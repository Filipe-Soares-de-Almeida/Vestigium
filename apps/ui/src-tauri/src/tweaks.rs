use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use vestigium_collect::{
    RESTORE_CREATED, WindowsSettings, has_battery, hiberfile_bytes, monitor_count, physical_drive,
    read_setting, restore_events, sha256_hex, startup_entries, system_drive,
};
use vestigium_core::{
    Advice, Applier, Helped, PlanItem, Status, TWEAK_PLAN_VERSION, TWEAKS, TweakChange, TweakId,
    TweakInputs, TweakJournal, TweakPlan, advise, changed_outside, check_undo, helped, undo,
};
use vestigium_ipc::{Request, Response};
use vestigium_store::TweakChangeRow;
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

use crate::agent::{Link, UiError, refused};
use crate::capture::{
    HELPER_UNAVAILABLE, TWEAK_CEILING, launch_elevated, sibling_helper, wait_result,
};
use crate::history::{History, TweakHistory};

const DAY_MS: i64 = 24 * 3_600_000;
/// How far back the rules look for signals.
pub const EVIDENCE_MS: i64 = 30 * DAY_MS;
/// How far back the page lists changes; changes still in place are listed whatever their age.
const HISTORY_SHOWN_MS: i64 = 365 * DAY_MS;

/// One change in the history, with whether it helped and whether Windows still holds it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TweakRowDto {
    pub id: i64,
    pub applied_ms: i64,
    pub undone_ms: Option<i64>,
    pub applier: Applier,
    pub change: TweakChange,
    pub helped: Option<Helped>,
    /// Windows now holds something other than what this change wrote.
    pub outside: bool,
}

/// The tweaks page: every tweak with its signals, the history, and Windows' last restore point.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TweaksDto {
    pub advice: Vec<Advice>,
    pub history: Vec<TweakRowDto>,
    pub last_restore_point_ms: Option<i64>,
}

/// A tweak the person chose, by name; its changes are worked out again here.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TweakPick {
    pub tweak: TweakId,
    pub subject: Option<String>,
}

/// What applying or undoing returned: the history rows written and the helper's exit code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TweaksApplied {
    pub ids: Vec<i64>,
    pub exit_code: Option<u32>,
}

/// What each setting holds now and the history the rules read.
fn inputs(found: &TweakHistory, now_ms: i64) -> TweakInputs {
    let startup = startup_entries();
    let mut values = Vec::new();
    for setting in TWEAKS.iter().flat_map(|t| t.settings()) {
        if let Some(value) = read_setting(&setting) {
            values.push((setting, value));
        }
    }
    for entry in &startup {
        let approval = entry.approval();
        if let Some(value) = read_setting(&approval) {
            values.push((approval, value));
        }
    }
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let system_disk = system_drive().and_then(|number| {
        found
            .machine
            .as_ref()?
            .disks
            .iter()
            .find(|d| physical_drive(&d.name) == Some(number))
            .map(|d| d.name.clone())
    });
    TweakInputs {
        now_ms,
        evidence_from_ms: now_ms - EVIDENCE_MS,
        values,
        startup,
        hiberfile_bytes: hiberfile_bytes(),
        system_root: format!(r"{drive}\"),
        system_disk,
        monitors: monitor_count(),
        battery: has_battery(),
        findings: found.findings.clone(),
        occurrences: found.occurrences.clone(),
        episodes: found.episodes.clone(),
        captures: found.captures.clone(),
        events: found.events.clone(),
        tests: found.tests.clone(),
        covered: found.covered.clone(),
    }
}

pub fn read(history: &mut History, now_ms: i64) -> Result<TweaksDto, UiError> {
    let found = history.tweak_history(now_ms, EVIDENCE_MS, HISTORY_SHOWN_MS)?;
    let inputs = inputs(&found, now_ms);
    let history = found
        .changes
        .iter()
        .map(|row| TweakRowDto {
            id: row.id,
            applied_ms: row.applied_ms,
            undone_ms: row.undone_ms,
            applier: row.applier,
            change: row.change.clone(),
            helped: helped(row.change.tweak, row.applied_ms, &inputs),
            outside: row.undone_ms.is_none() && row.change.undo_of.is_none() && changed_since(row),
        })
        .collect();
    let last_restore_point_ms = restore_events(now_ms - 30 * DAY_MS)
        .unwrap_or_default()
        .iter()
        .filter(|e| e.id == RESTORE_CREATED)
        .map(|e| e.ts_ms)
        .max();
    Ok(TweaksDto {
        advice: advise(&inputs),
        history,
        last_restore_point_ms,
    })
}

/// Whether a setting this change wrote now holds another value.
fn changed_since(row: &TweakChangeRow) -> bool {
    row.change
        .results
        .iter()
        .filter(|r| r.error.is_none())
        .any(|r| read_setting(&r.change.setting).is_some_and(|now| now != r.change.to))
}

/// Runs the helper elevated with `args` and waits for it; its exit code, when it has one.
pub fn run_helper(args: &str) -> Result<Option<u32>, UiError> {
    let helper = sibling_helper().ok_or_else(|| UiError::Failed(HELPER_UNAVAILABLE.into()))?;
    let process = launch_elevated(&helper, args)?;
    wait_result(&process, TWEAK_CEILING, &AtomicBool::new(false))
}

/// Applies the tweaks picked: the user's own through the watcher, the rest in one elevated batch
/// that `launch` runs. The history is locked only while it is read, never while `launch` waits.
pub fn apply(
    history: &Mutex<History>,
    link: &Link,
    picks: &[TweakPick],
    without_restore_point: bool,
    now_ms: i64,
    launch: impl FnOnce(&str) -> Result<Option<u32>, UiError>,
) -> Result<TweaksApplied, UiError> {
    let found = lock(history).tweak_history(now_ms, EVIDENCE_MS, HISTORY_SHOWN_MS)?;
    let advice = advise(&inputs(&found, now_ms));
    let items: Vec<(PlanItem, Applier)> = picks
        .iter()
        .filter_map(|pick| {
            let a = advice
                .iter()
                .find(|a| a.tweak == pick.tweak && a.subject == pick.subject)?;
            (a.status == Status::Ready).then(|| {
                let item = PlanItem {
                    tweak: a.tweak,
                    subject: a.subject.clone(),
                    undo_of: None,
                    undo_journal: None,
                    changes: a.changes.clone(),
                };
                (item, a.applier)
            })
        })
        .collect();
    carry(history, link, items, without_restore_point, now_ms, launch)
}

/// Puts back what one change wrote, by the way it was written; an elevated change the journal
/// does not hold is refused, unless it predates the journal.
pub fn undo_change(
    history: &Mutex<History>,
    link: &Link,
    journal: &impl TweakJournal,
    id: i64,
    without_restore_point: bool,
    now_ms: i64,
    launch: impl FnOnce(&str) -> Result<Option<u32>, UiError>,
) -> Result<TweaksApplied, UiError> {
    let row = lock(history)
        .tweak_change(id)?
        .ok_or_else(|| UiError::Failed(format!("no change {id}")))?;
    if row.undone_ms.is_some() || row.change.undo_of.is_some() {
        return Err(UiError::Failed(format!("change {id} cannot be undone")));
    }
    let written = row.change.written();
    let item = PlanItem {
        tweak: row.change.tweak,
        subject: row.change.subject.clone(),
        undo_of: Some(id),
        undo_journal: row
            .change
            .journal
            .filter(|_| row.applier == Applier::Elevated),
        changes: undo(&written),
    };
    if row.applier == Applier::Elevated {
        check_undo(&item, journal).map_err(UiError::Failed)?;
    }
    if changed_outside(&written, &mut WindowsSettings) {
        return Err(UiError::ChangedSince);
    }
    if written.is_empty() {
        return Ok(TweaksApplied {
            ids: Vec::new(),
            exit_code: None,
        });
    }
    carry(
        history,
        link,
        vec![(item, row.applier)],
        without_restore_point,
        now_ms,
        launch,
    )
}

fn carry(
    history: &Mutex<History>,
    link: &Link,
    items: Vec<(PlanItem, Applier)>,
    without_restore_point: bool,
    now_ms: i64,
    launch: impl FnOnce(&str) -> Result<Option<u32>, UiError>,
) -> Result<TweaksApplied, UiError> {
    let mut ids = Vec::new();
    let (user, elevated): (Vec<_>, Vec<_>) =
        items.into_iter().partition(|(_, a)| *a == Applier::User);
    for (item, _) in user {
        match link.call(&Request::ApplyUserTweak { item })? {
            Response::TweakApplied { id, .. } => ids.push(id),
            other => return Err(refused(other)),
        }
    }
    let mut exit_code = None;
    if !elevated.is_empty() {
        let plan = TweakPlan {
            items: elevated.into_iter().map(|(item, _)| item).collect(),
            without_restore_point,
        };
        let data_dir = lock(history).data_dir();
        let path = data_dir
            .map(|dir| dir.join("tweaks").join(format!("{now_ms}.plan")))
            .ok_or_else(|| UiError::Failed("no data folder".into()))?;
        let held = write_plan(&path, &plan)?;
        let args = format!(
            "tweak --plan \"{}\" --sha256 {}",
            path.display(),
            held.sha256
        );
        exit_code = launch(&args)?;
        drop(held);
        if let Ok(Response::Imported { tweaks, .. }) = link.call(&Request::ImportResults) {
            ids.extend(tweaks);
        }
    }
    Ok(TweaksApplied { ids, exit_code })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A plan file kept open without write sharing, with the SHA-256 of the bytes written to it.
struct HeldPlan {
    _file: File,
    sha256: String,
}

/// Opens the plan at `path` so nobody can write to it, and checks that it holds `written`.
fn hold_plan(path: &Path, written: &[u8]) -> Result<HeldPlan, UiError> {
    let failed = |err: &dyn std::fmt::Display| UiError::Failed(err.to_string());
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0)
        .open(path)
        .map_err(|e| failed(&e))?;
    let mut found = Vec::new();
    file.read_to_end(&mut found).map_err(|e| failed(&e))?;
    if found != written {
        return Err(UiError::Failed(
            "the plan file changed after it was written".into(),
        ));
    }
    Ok(HeldPlan {
        _file: file,
        sha256: sha256_hex(written).map_err(|e| failed(&e))?,
    })
}

/// Writes the plan and holds the file until the helper has read it.
fn write_plan(path: &Path, plan: &TweakPlan) -> Result<HeldPlan, UiError> {
    let failed = |err: &dyn std::fmt::Display| UiError::Failed(err.to_string());
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| failed(&e))?;
    }
    let mut bytes = vec![TWEAK_PLAN_VERSION];
    bytes.extend(postcard::to_allocvec(plan).map_err(|e| failed(&e))?);
    std::fs::write(path, &bytes).map_err(|e| failed(&e))?;
    hold_plan(path, &bytes)
}

#[cfg(test)]
mod tests {
    use vestigium_core::{IntentSetting, JournalEntry, SettingChange, SettingResult, Value};

    use super::*;

    #[test]
    fn the_history_is_free_while_the_helper_runs() {
        let dir = tempfile::tempdir().unwrap();
        let history = Mutex::new(History::new(Some(dir.path().join("vestigium.db"))));
        let link = Link::new(None);
        let setting = TweakId::GameDvr.settings().remove(0);
        let item = PlanItem {
            tweak: TweakId::GameDvr,
            subject: None,
            undo_of: None,
            undo_journal: None,
            changes: vec![SettingChange {
                setting,
                from: Value::Dword(1),
                to: Value::Dword(0),
            }],
        };
        let mut was_free = false;

        let applied = carry(
            &history,
            &link,
            vec![(item, Applier::Elevated)],
            false,
            7,
            |_| {
                was_free = history.try_lock().is_ok();
                Ok(Some(0))
            },
        )
        .unwrap();

        assert!(was_free, "the history stayed locked while the helper ran");
        assert_eq!(applied.exit_code, Some(0));
    }

    #[test]
    fn a_held_plan_cannot_be_rewritten_and_must_match_what_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("1.plan");
        std::fs::write(&path, b"abc").unwrap();

        assert!(hold_plan(&path, b"abd").is_err(), "swapped before the hold");

        let held = hold_plan(&path, b"abc").unwrap();
        assert_eq!(
            held.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(std::fs::write(&path, b"changed").is_err());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"abc",
            "readers still get in"
        );
        drop(held);
        assert!(std::fs::write(&path, b"changed").is_ok());
    }

    /// A journal held in memory; ids count from 1.
    struct Journal(Vec<JournalEntry>);

    impl TweakJournal for Journal {
        fn entry(&self, id: u64) -> Result<Option<JournalEntry>, String> {
            let at = usize::try_from(id).ok().and_then(|id| id.checked_sub(1));
            Ok(at.and_then(|at| self.0.get(at)).cloned())
        }

        fn record(&mut self, _: &JournalEntry) -> Result<u64, String> {
            panic!("the window only reads the journal");
        }
    }

    /// The Mpo tweak taking its setting from absent to 5, as a row of the history and a journal entry.
    fn mpo_row(journal: Option<u64>) -> (TweakChange, Journal) {
        let setting = TweakId::Mpo.settings().remove(0);
        let change = TweakChange {
            tweak: TweakId::Mpo,
            subject: None,
            undo_of: None,
            results: vec![SettingResult {
                change: SettingChange {
                    setting: setting.clone(),
                    from: Value::Absent,
                    to: Value::Dword(5),
                },
                error: None,
            }],
            restore_point: None,
            journal,
        };
        let entry = JournalEntry {
            ts_ms: 1,
            tweak: TweakId::Mpo,
            subject: None,
            undoes: None,
            settings: vec![IntentSetting {
                setting,
                before: Some(Value::Absent),
                to: Value::Dword(5),
            }],
        };
        (change, Journal(vec![entry]))
    }

    /// Records `change` as an elevated row and tries to undo it; the plan the helper was launched
    /// with, when it was.
    fn try_undo(
        change: &TweakChange,
        journal: &Journal,
    ) -> (Result<TweaksApplied, UiError>, Option<TweakPlan>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vestigium.db");
        let id = vestigium_store::Store::open(&path)
            .unwrap()
            .record_tweak_change(change, 10, Applier::Elevated)
            .unwrap();
        let history = Mutex::new(History::new(Some(path)));
        let mut launched = None;
        let undone = undo_change(&history, &Link::new(None), journal, id, false, 20, |args| {
            let path = args.split('"').nth(1).unwrap();
            let bytes = std::fs::read(path).unwrap();
            launched = postcard::from_bytes::<TweakPlan>(&bytes[1..]).ok();
            Ok(Some(0))
        });
        (undone, launched)
    }

    #[test]
    fn an_undo_the_journal_does_not_hold_never_reaches_the_helper() {
        let (change, journal) = mpo_row(Some(9));

        let (undone, launched) = try_undo(&change, &journal);

        assert!(matches!(undone, Err(UiError::Failed(_))), "{undone:?}");
        assert!(
            launched.is_none(),
            "no permission prompt for a change nobody recorded"
        );
    }

    #[test]
    fn an_undo_of_another_tweaks_entry_never_reaches_the_helper() {
        let (mut change, mut journal) = mpo_row(Some(1));
        journal.0[0].tweak = TweakId::GameDvr;
        change.results[0].change.to = Value::Dword(5);

        let (undone, launched) = try_undo(&change, &journal);

        assert!(matches!(undone, Err(UiError::Failed(_))), "{undone:?}");
        assert!(launched.is_none());
    }

    #[test]
    fn an_undo_of_a_journaled_change_names_its_entry_in_the_plan() {
        let (change, journal) = mpo_row(Some(1));

        let (undone, launched) = try_undo(&change, &journal);

        assert!(undone.is_ok(), "{undone:?}");
        let item = &launched.unwrap().items[0];
        assert_eq!(item.undo_journal, Some(1));
        assert_eq!(item.undo_of, Some(1));
    }

    #[test]
    fn an_undo_of_a_change_made_before_the_journal_still_reaches_the_helper() {
        let (change, journal) = mpo_row(None);

        let (undone, launched) = try_undo(&change, &journal);

        assert!(undone.is_ok(), "{undone:?}");
        assert_eq!(launched.unwrap().items[0].undo_journal, None);
    }
}
