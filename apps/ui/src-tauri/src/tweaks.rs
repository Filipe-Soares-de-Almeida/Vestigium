use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use vestigium_collect::{
    RESTORE_CREATED, WindowsSettings, has_battery, hiberfile_bytes, monitor_count, physical_drive,
    read_setting, restore_events, sha256_hex, startup_entries, system_drive,
};
use vestigium_core::{
    Advice, Applier, Helped, PlanItem, Status, TWEAK_PLAN_VERSION, TWEAKS, TweakChange, TweakId,
    TweakInputs, TweakPlan, advise, changed_outside, helped, undo,
};
use vestigium_ipc::{Request, Response};
use vestigium_store::TweakChangeRow;
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;

use crate::agent::{Link, UiError, refused};
use crate::capture::{launch_elevated, sibling_helper};
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

/// Applies the tweaks picked: the user's own through the watcher, the rest in one elevated batch.
pub fn apply(
    history: &mut History,
    link: &Link,
    picks: &[TweakPick],
    without_restore_point: bool,
    now_ms: i64,
) -> Result<TweaksApplied, UiError> {
    let found = history.tweak_history(now_ms, EVIDENCE_MS, HISTORY_SHOWN_MS)?;
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
                    changes: a.changes.clone(),
                };
                (item, a.applier)
            })
        })
        .collect();
    carry(history, link, items, without_restore_point, now_ms)
}

/// Puts back what one change wrote, by the same way it was written.
pub fn undo_change(
    history: &mut History,
    link: &Link,
    id: i64,
    without_restore_point: bool,
    now_ms: i64,
) -> Result<TweaksApplied, UiError> {
    let row = history
        .tweak_change(id)?
        .ok_or_else(|| UiError::Failed(format!("no change {id}")))?;
    if row.undone_ms.is_some() || row.change.undo_of.is_some() {
        return Err(UiError::Failed(format!("change {id} cannot be undone")));
    }
    let written = row.change.written();
    if changed_outside(&written, &mut WindowsSettings) {
        return Err(UiError::ChangedSince);
    }
    if written.is_empty() {
        return Ok(TweaksApplied {
            ids: Vec::new(),
            exit_code: None,
        });
    }
    let item = PlanItem {
        tweak: row.change.tweak,
        subject: row.change.subject.clone(),
        undo_of: Some(id),
        changes: undo(&written),
    };
    carry(
        history,
        link,
        vec![(item, row.applier)],
        without_restore_point,
        now_ms,
    )
}

fn carry(
    history: &History,
    link: &Link,
    items: Vec<(PlanItem, Applier)>,
    without_restore_point: bool,
    now_ms: i64,
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
        let path = history
            .data_dir()
            .map(|dir| dir.join("tweaks").join(format!("{now_ms}.plan")))
            .ok_or_else(|| UiError::Failed("no data folder".into()))?;
        let held = write_plan(&path, &plan)?;
        let helper = sibling_helper()
            .ok_or_else(|| UiError::Failed("the helper is not next to the window".into()))?;
        let args = format!(
            "tweak --plan \"{}\" --sha256 {}",
            path.display(),
            held.sha256
        );
        exit_code = launch_elevated(&helper, &args)?.wait();
        drop(held);
        if let Ok(Response::Imported { tweaks, .. }) = link.call(&Request::ImportResults) {
            ids.extend(tweaks);
        }
    }
    Ok(TweaksApplied { ids, exit_code })
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
    use super::*;

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
}
