use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use vestigium_collect::{
    RESTORE_CREATED, RESTORE_SKIPPED, WindowsSettings, restore_events, wall_ms,
};
use vestigium_core::{
    Applier, PlanItem, RestorePoint, SettingResult, SettingStore, TWEAK_PLAN_VERSION, TweakChange,
    TweakPlan, TweakResults, carry_out, check_item,
};
use windows::Win32::Foundation::ERROR_SERVICE_DISABLED;
use windows::Win32::System::Restore::{
    BEGIN_SYSTEM_CHANGE, END_SYSTEM_CHANGE, MODIFY_SETTINGS, RESTOREPOINTINFO_EVENT_TYPE,
    RESTOREPOINTINFOW, SRSetRestorePointW, STATEMGRSTATUS,
};

use crate::{job, output};

/// Most items one plan may hold.
const MOST_ITEMS: usize = 32;
/// Win32 error recorded for an item the catalog refuses, or one left out for lack of a restore point.
pub const ERROR_REFUSED: u32 = 5;
/// How long System Restore may take to log what it did with a point.
const RESTORE_LOG_WAIT: Duration = Duration::from_secs(10);
const DAY_MS: i64 = 24 * 60 * 60_000;

/// Carries out a `.plan` of elevated tweaks and writes what happened to a `.tweaked` file beside it.
pub fn run(plan_path: &Path) -> ExitCode {
    if output::check_existing(plan_path, "tweaks", "plan").is_err() {
        return ExitCode::from(job::REFUSED);
    }
    let Some(plan) = std::fs::read(plan_path).ok().and_then(|b| read_plan(&b)) else {
        return ExitCode::from(job::REFUSED);
    };
    let Ok(mut file) = output::create(&plan_path.with_extension("tweaked"), "tweaks", "tweaked")
    else {
        return ExitCode::from(job::REFUSED);
    };
    let mut point = SystemRestore::default();
    let results = apply(&plan, &mut WindowsSettings, || point.begin());
    point.end();
    let mut bytes = vec![TWEAK_PLAN_VERSION];
    match postcard::to_allocvec(&results) {
        Ok(body) => bytes.extend(body),
        Err(_) => return ExitCode::from(job::WRITE_FAILED),
    }
    if file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        return ExitCode::from(job::WRITE_FAILED);
    }
    ExitCode::SUCCESS
}

/// A plan file: its version byte, then the plan; `None` for another version or too many items.
pub fn read_plan(bytes: &[u8]) -> Option<TweakPlan> {
    let (&version, body) = bytes.split_first()?;
    if version != TWEAK_PLAN_VERSION {
        return None;
    }
    let plan: TweakPlan = postcard::from_bytes(body).ok()?;
    (plan.items.len() <= MOST_ITEMS).then_some(plan)
}

/// Checks every item, asks for one restore point when something will be written, then carries
/// out the items that passed in order. Without a point, nothing is written unless the plan says so.
pub fn apply(
    plan: &TweakPlan,
    store: &mut impl SettingStore,
    restore_point: impl FnOnce() -> RestorePoint,
) -> TweakResults {
    let checked: Vec<bool> = plan
        .items
        .iter()
        .map(|item| check_item(item, Applier::Elevated).is_ok())
        .collect();
    let point = checked.contains(&true).then(restore_point);
    let blocked = matches!(point, Some(RestorePoint::Off | RestorePoint::Failed { .. }))
        && !plan.without_restore_point;
    let changes = plan
        .items
        .iter()
        .zip(&checked)
        .map(|(item, &ok)| {
            if ok && !blocked {
                TweakChange {
                    restore_point: point,
                    ..carry_out(item, store)
                }
            } else {
                refused(item, point.filter(|_| ok))
            }
        })
        .collect();
    TweakResults {
        ts_ms: wall_ms(),
        changes,
    }
}

fn refused(item: &PlanItem, point: Option<RestorePoint>) -> TweakChange {
    TweakChange {
        tweak: item.tweak,
        subject: item.subject.clone(),
        undo_of: item.undo_of,
        results: item
            .changes
            .iter()
            .take(1)
            .map(|change| SettingResult {
                change: change.clone(),
                error: Some(ERROR_REFUSED),
            })
            .collect(),
        restore_point: point,
    }
}

/// One restore point around a batch of changes, told apart by its description.
#[derive(Default)]
struct SystemRestore {
    sequence: Option<i64>,
}

impl SystemRestore {
    fn begin(&mut self) -> RestorePoint {
        let started = wall_ms();
        let description = format!("Vestigium {started}");
        let (ok, status, sequence) = set_point(BEGIN_SYSTEM_CHANGE, 0, &description);
        if !ok {
            return if status == ERROR_SERVICE_DISABLED.0 {
                RestorePoint::Off
            } else {
                RestorePoint::Failed { error: status }
            };
        }
        self.sequence = Some(sequence);
        let waiting = Instant::now();
        while waiting.elapsed() < RESTORE_LOG_WAIT {
            let events = restore_events(started - 30 * DAY_MS).unwrap_or_default();
            if let Some(ours) = events
                .iter()
                .find(|e| e.ts_ms >= started - 1000 && e.description == description)
            {
                return match ours.id {
                    RESTORE_CREATED => RestorePoint::Created,
                    RESTORE_SKIPPED => RestorePoint::Reused {
                        at_ms: events
                            .iter()
                            .filter(|e| e.id == RESTORE_CREATED)
                            .map(|e| e.ts_ms)
                            .max(),
                    },
                    _ => RestorePoint::Unconfirmed,
                };
            }
            sleep(Duration::from_millis(500));
        }
        RestorePoint::Unconfirmed
    }

    fn end(&mut self) {
        if let Some(sequence) = self.sequence.take() {
            let _ = set_point(END_SYSTEM_CHANGE, sequence, "");
        }
    }
}

/// One SRSetRestorePointW call: whether it succeeded, its status and the sequence number it gave.
fn set_point(
    kind: RESTOREPOINTINFO_EVENT_TYPE,
    sequence: i64,
    description: &str,
) -> (bool, u32, i64) {
    let mut text = [0u16; 256];
    for (slot, unit) in text.iter_mut().zip(description.encode_utf16().take(255)) {
        *slot = unit;
    }
    let info = RESTOREPOINTINFOW {
        dwEventType: kind,
        dwRestorePtType: MODIFY_SETTINGS,
        llSequenceNumber: sequence,
        szDescription: text,
    };
    let mut status = STATEMGRSTATUS::default();
    // SAFETY: both structs are the ones the call reads and fills.
    let ok = unsafe { SRSetRestorePointW(&info, &mut status) };
    (ok.as_bool(), status.nStatus.0, status.llSequenceNumber)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use vestigium_core::{Hive, Setting, SettingChange, TweakId, Value};

    use super::*;

    struct Fake {
        values: Vec<(Setting, Value)>,
        writes: usize,
    }

    impl SettingStore for Fake {
        fn read(&mut self, setting: &Setting) -> Option<Value> {
            self.values
                .iter()
                .find(|(s, _)| s == setting)
                .map(|(_, v)| v.clone())
        }

        fn write(&mut self, _: &Setting, _: &Value) -> Result<(), u32> {
            self.writes += 1;
            Ok(())
        }
    }

    fn mpo() -> (Setting, PlanItem) {
        let setting = TweakId::Mpo.settings()[0].clone();
        let item = PlanItem {
            tweak: TweakId::Mpo,
            subject: None,
            undo_of: None,
            changes: vec![SettingChange {
                setting: setting.clone(),
                from: Value::Absent,
                to: Value::Dword(5),
            }],
        };
        (setting, item)
    }

    fn game_dvr() -> PlanItem {
        PlanItem {
            tweak: TweakId::GameDvr,
            subject: None,
            undo_of: None,
            changes: vec![SettingChange {
                setting: TweakId::GameDvr.settings()[0].clone(),
                from: Value::Dword(1),
                to: Value::Dword(0),
            }],
        }
    }

    #[test]
    fn refuses_a_key_outside_the_catalog_and_goes_on() {
        let (setting, good) = mpo();
        let mut bad = good.clone();
        bad.changes[0].setting = Setting::Dword {
            hive: Hive::LocalMachine,
            key: r"SOFTWARE\Elsewhere".into(),
            name: "OverlayTestMode".into(),
        };
        let plan = TweakPlan {
            items: vec![bad, game_dvr(), good],
            without_restore_point: false,
        };
        let mut store = Fake {
            values: vec![(setting, Value::Absent)],
            writes: 0,
        };
        let results = apply(&plan, &mut store, || RestorePoint::Created);
        let errors: Vec<_> = results.changes.iter().map(|c| c.results[0].error).collect();
        assert_eq!(errors, [Some(ERROR_REFUSED), Some(ERROR_REFUSED), None]);
        assert_eq!(store.writes, 1);
        assert_eq!(
            results.changes[2].restore_point,
            Some(RestorePoint::Created)
        );
    }

    #[test]
    fn nothing_is_written_without_a_restore_point_unless_agreed() {
        let (setting, item) = mpo();
        let mut plan = TweakPlan {
            items: vec![item],
            without_restore_point: false,
        };
        let mut store = Fake {
            values: vec![(setting, Value::Absent)],
            writes: 0,
        };
        let results = apply(&plan, &mut store, || RestorePoint::Off);
        assert_eq!(results.changes[0].results[0].error, Some(ERROR_REFUSED));
        assert_eq!(store.writes, 0);
        plan.without_restore_point = true;
        let results = apply(&plan, &mut store, || RestorePoint::Off);
        assert_eq!(results.changes[0].results[0].error, None);
        assert_eq!(store.writes, 1);
    }

    #[test]
    fn no_point_is_asked_when_every_item_is_refused() {
        let plan = TweakPlan {
            items: vec![game_dvr()],
            without_restore_point: false,
        };
        let mut store = Fake {
            values: Vec::new(),
            writes: 0,
        };
        let results = apply(&plan, &mut store, || panic!("no point for refused items"));
        assert_eq!(results.changes[0].restore_point, None);
    }

    #[test]
    fn read_plan_checks_the_version() {
        let (_, item) = mpo();
        let plan = TweakPlan {
            items: vec![item],
            without_restore_point: false,
        };
        let mut bytes = vec![TWEAK_PLAN_VERSION];
        bytes.extend(postcard::to_allocvec(&plan).unwrap());
        assert_eq!(read_plan(&bytes), Some(plan));
        bytes[0] = TWEAK_PLAN_VERSION + 1;
        assert_eq!(read_plan(&bytes), None);
        assert_eq!(read_plan(&[]), None);
    }

    #[test]
    fn plan_with_a_link_on_the_way_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("Elsewhere");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("1.plan"), [TWEAK_PLAN_VERSION]).unwrap();
        let data = dir.path().join("Vestigium").join("data");
        std::fs::create_dir_all(&data).unwrap();
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(data.join("tweaks"))
            .arg(&real)
            .output()
            .unwrap();
        assert!(made.status.success(), "could not make a junction");
        let through = data.join("tweaks").join("1.plan");
        assert!(output::check_existing(&through, "tweaks", "plan").is_err());
    }
}
