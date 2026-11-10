use std::io::{Read, Write};
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
use windows::Win32::Foundation::{E_FAIL, ERROR_SERVICE_DISABLED};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    CoInitializeSecurity, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL,
    RPC_C_AUTHN_LEVEL_DEFAULT, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Restore::{BEGIN_SYSTEM_CHANGE, MODIFY_SETTINGS};
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
use windows::Win32::System::Variant::{VARIANT, VT_BSTR, VT_I4, VariantClear};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_FLAG_RETURN_WBEM_COMPLETE, WBEM_GENERIC_FLAG_TYPE,
    WBEM_INFINITE, WbemLocator,
};
use windows::core::{BSTR, PCWSTR, w};

use crate::{job, output};

/// Most items one plan may hold.
const MOST_ITEMS: usize = 32;
/// Win32 error recorded for an item the catalog refuses, or one left out for lack of a restore point.
pub const ERROR_REFUSED: u32 = 5;
/// How long System Restore may take to log what it did with a point.
const RESTORE_LOG_WAIT: Duration = Duration::from_secs(10);
/// How long after "created" a point is looked for in the list: Windows drops a failed one within ~6 s.
const RESTORE_SETTLE: Duration = Duration::from_secs(10);
const DAY_MS: i64 = 24 * 60 * 60_000;
/// Win32 error recorded for a point Windows logged as created and then dropped (ERROR_NOT_FOUND).
const ERROR_RESTORE_DROPPED: u32 = 1168;

/// Carries out a `.plan` of elevated tweaks and writes what happened to a `.tweaked` file beside it.
pub fn run(plan_path: &Path) -> ExitCode {
    let Ok(mut input) = output::open_existing(plan_path, "tweaks", "plan") else {
        return ExitCode::from(job::REFUSED);
    };
    let mut bytes = Vec::new();
    if input.read_to_end(&mut bytes).is_err() {
        return ExitCode::from(job::REFUSED);
    }
    let Some(plan) = read_plan(&bytes) else {
        return ExitCode::from(job::REFUSED);
    };
    let Ok(mut file) = output::create(&plan_path.with_extension("tweaked"), "tweaks", "tweaked")
    else {
        return ExitCode::from(job::REFUSED);
    };
    let results = apply(&plan, &mut WindowsSettings, ask_restore_point);
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
    let blocked = matches!(
        point,
        Some(RestorePoint::Off | RestorePoint::Failed { .. } | RestorePoint::Unconfirmed)
    ) && !plan.without_restore_point;
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

/// One restore point before a batch of changes, told apart by its description.
fn ask_restore_point() -> RestorePoint {
    let started = wall_ms();
    let description = format!("Vestigium {started}");
    match create_restore_point(&description) {
        Ok(0) => {}
        Ok(code) if code == ERROR_SERVICE_DISABLED.0 => return RestorePoint::Off,
        Ok(code) => return RestorePoint::Failed { error: code },
        Err(err) => {
            return RestorePoint::Failed {
                error: err.code().0 as u32,
            };
        }
    }
    let waiting = Instant::now();
    while waiting.elapsed() < RESTORE_LOG_WAIT {
        let events = restore_events(started - 30 * DAY_MS).unwrap_or_default();
        if let Some(ours) = events
            .iter()
            .find(|e| e.ts_ms >= started - 1000 && e.description == description)
        {
            let listed = if ours.id == RESTORE_CREATED {
                sleep(RESTORE_SETTLE);
                restore_point_listed(&description)
            } else {
                Ok(false)
            };
            let last_created = events
                .iter()
                .filter(|e| e.id == RESTORE_CREATED)
                .map(|e| e.ts_ms)
                .max();
            return settle(ours.id, listed, last_created);
        }
        sleep(Duration::from_millis(500));
    }
    RestorePoint::Unconfirmed
}

/// What the log event and the list query say about a point: a point logged as created but absent
/// from the list was dropped, and a list that cannot be read leaves it unconfirmed.
fn settle<E>(event_id: u16, listed: Result<bool, E>, last_created: Option<i64>) -> RestorePoint {
    match event_id {
        RESTORE_CREATED => match listed {
            Ok(true) => RestorePoint::Created,
            Ok(false) => RestorePoint::Failed {
                error: ERROR_RESTORE_DROPPED,
            },
            Err(_) => RestorePoint::Unconfirmed,
        },
        RESTORE_SKIPPED => RestorePoint::Reused {
            at_ms: last_created,
        },
        _ => RestorePoint::Unconfirmed,
    }
}

fn int_variant(value: i32) -> VARIANT {
    let mut v = VARIANT::default();
    // SAFETY: sets the tag and the member it names on a fresh VARIANT.
    unsafe {
        (*v.Anonymous.Anonymous).vt = VT_I4;
        (*v.Anonymous.Anonymous).Anonymous.lVal = value;
    }
    v
}

fn text_variant(value: &str) -> VARIANT {
    let mut v = VARIANT::default();
    // SAFETY: sets the tag and the member it names on a fresh VARIANT; VariantClear frees it.
    unsafe {
        (*v.Anonymous.Anonymous).vt = VT_BSTR;
        (*v.Anonymous.Anonymous).Anonymous.bstrVal = std::mem::ManuallyDrop::new(BSTR::from(value));
    }
    v
}

/// System Restore's WMI namespace, with the call security its methods need.
fn restore_services() -> windows::core::Result<IWbemServices> {
    // SAFETY: COM calls on interfaces this function owns.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = CoInitializeSecurity(
            None,
            -1,
            None,
            None,
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
            None,
        );
        let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
        let empty = BSTR::new();
        let services = locator.ConnectServer(
            &BSTR::from(r"ROOT\DEFAULT"),
            &empty,
            &empty,
            &empty,
            0,
            &empty,
            None,
        )?;
        CoSetProxyBlanket(
            &services,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )?;
        Ok(services)
    }
}

/// Whether System Restore lists a point with this description.
pub fn restore_point_listed(description: &str) -> windows::core::Result<bool> {
    let query =
        format!("SELECT SequenceNumber FROM SystemRestore WHERE Description = '{description}'");
    // SAFETY: COM calls on interfaces this function owns.
    unsafe {
        let found = restore_services()?.ExecQuery(
            &BSTR::from("WQL"),
            &BSTR::from(query),
            WBEM_GENERIC_FLAG_TYPE(WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0),
            None,
        )?;
        let mut row = [None];
        let mut returned = 0u32;
        found.Next(WBEM_INFINITE, &mut row, &mut returned).ok()?;
        Ok(returned > 0)
    }
}

/// Asks System Restore for a point through `SystemRestore.CreateRestorePoint` and returns its
/// ReturnValue. SRSetRestorePointW from this process logs the point as created and drops it.
pub fn create_restore_point(description: &str) -> windows::core::Result<u32> {
    let services = restore_services()?;
    // SAFETY: COM calls on interfaces this function owns; every VARIANT is cleared after use.
    unsafe {
        let class_name = BSTR::from("SystemRestore");
        let mut class: Option<IWbemClassObject> = None;
        services.GetObject(
            &class_name,
            WBEM_FLAG_RETURN_WBEM_COMPLETE,
            None,
            Some(&mut class),
            None,
        )?;
        let mut input: Option<IWbemClassObject> = None;
        class.ok_or(E_FAIL)?.GetMethod(
            w!("CreateRestorePoint"),
            0,
            &mut input,
            std::ptr::null_mut(),
        )?;
        let params = input.ok_or(E_FAIL)?.SpawnInstance(0)?;
        let mut text = text_variant(description);
        let put = params.Put(w!("Description"), 0, &text, 0);
        VariantClear(&mut text)?;
        put?;
        params.Put(
            w!("RestorePointType"),
            0,
            &int_variant(MODIFY_SETTINGS.0 as i32),
            0,
        )?;
        params.Put(
            w!("EventType"),
            0,
            &int_variant(BEGIN_SYSTEM_CHANGE.0 as i32),
            0,
        )?;
        let mut output: Option<IWbemClassObject> = None;
        services.ExecMethod(
            &class_name,
            &BSTR::from("CreateRestorePoint"),
            WBEM_FLAG_RETURN_WBEM_COMPLETE,
            None,
            &params,
            Some(&mut output),
            None,
        )?;
        let mut value = VARIANT::default();
        output
            .ok_or(E_FAIL)?
            .Get(w!("ReturnValue"), 0, &mut value, None, None)?;
        let code = (*value.Anonymous.Anonymous).Anonymous.lVal as u32;
        VariantClear(&mut value)?;
        Ok(code)
    }
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
    fn a_point_the_list_does_not_hold_is_failed() {
        assert_eq!(
            settle(RESTORE_CREATED, Ok::<bool, ()>(false), None),
            RestorePoint::Failed {
                error: ERROR_RESTORE_DROPPED
            }
        );
    }

    #[test]
    fn a_point_the_list_cannot_confirm_is_unconfirmed() {
        assert_eq!(
            settle(RESTORE_CREATED, Err::<bool, ()>(()), None),
            RestorePoint::Unconfirmed
        );
    }

    #[test]
    fn a_listed_point_is_created_and_a_skipped_one_is_reused() {
        assert_eq!(
            settle(RESTORE_CREATED, Ok::<bool, ()>(true), None),
            RestorePoint::Created
        );
        assert_eq!(
            settle(RESTORE_SKIPPED, Err::<bool, ()>(()), Some(7)),
            RestorePoint::Reused { at_ms: Some(7) }
        );
        assert_eq!(
            settle(1, Ok::<bool, ()>(true), None),
            RestorePoint::Unconfirmed
        );
    }

    #[test]
    fn an_unconfirmed_restore_point_blocks_the_write() {
        let (setting, item) = mpo();
        let mut plan = TweakPlan {
            items: vec![item],
            without_restore_point: false,
        };
        let mut store = Fake {
            values: vec![(setting, Value::Absent)],
            writes: 0,
        };
        let results = apply(&plan, &mut store, || RestorePoint::Unconfirmed);
        assert_eq!(results.changes[0].results[0].error, Some(ERROR_REFUSED));
        assert_eq!(
            results.changes[0].restore_point,
            Some(RestorePoint::Unconfirmed)
        );
        assert_eq!(store.writes, 0);
        plan.without_restore_point = true;
        apply(&plan, &mut store, || RestorePoint::Unconfirmed);
        assert_eq!(store.writes, 1);
    }

    #[test]
    fn a_created_or_reused_point_lets_the_write_through() {
        let (setting, item) = mpo();
        let plan = TweakPlan {
            items: vec![item],
            without_restore_point: false,
        };
        let mut store = Fake {
            values: vec![(setting, Value::Absent)],
            writes: 0,
        };
        apply(&plan, &mut store, || RestorePoint::Created);
        apply(&plan, &mut store, || RestorePoint::Reused { at_ms: None });
        assert_eq!(store.writes, 2);
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
