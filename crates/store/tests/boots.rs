use vestigium_core::Retention;
use vestigium_core::health::{BootFacts, StartupRun, TempSize};
use vestigium_store::Store;

const DAY: i64 = 24 * 60 * 60_000;
const RETENTION: Retention = Retention {
    samples_hours: 48,
    minutes_days: 90,
    records_days: 365,
};

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(&dir.path().join("vestigium.db")).unwrap()
}

fn boot(start: i64) -> BootFacts {
    BootFacts {
        kernel_start_ms: start,
        boot_type: Some(0),
        firmware_ms: Some(13_500),
        signin_ms: Some(start + 20_700),
        profile_start_ms: Some(start + 20_700),
        profile_end_ms: Some(start + 28_900),
        ..BootFacts::default()
    }
}

#[test]
fn boots_come_back_oldest_first_and_limited_to_the_latest() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    for day in [3, 1, 2] {
        store.put_boot(&boot(day * DAY)).unwrap();
    }
    let starts: Vec<i64> = store
        .boots(2)
        .unwrap()
        .iter()
        .map(|b| b.kernel_start_ms)
        .collect();
    assert_eq!(starts, [2 * DAY, 3 * DAY]);
    assert_eq!(store.last_boot_start().unwrap(), Some(3 * DAY));
}

#[test]
fn a_boot_read_again_replaces_the_stored_one() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store.put_boot(&boot(DAY)).unwrap();
    let mut later = boot(DAY);
    later.apps.push(StartupRun {
        command: "Example.exe".into(),
        start_ms: DAY + 47_000,
        end_ms: Some(DAY + 49_800),
    });
    store.put_boot(&later).unwrap();
    let boots = store.boots(10).unwrap();
    assert_eq!(boots, [later]);
}

#[test]
fn the_newest_temp_count_wins() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    assert_eq!(store.last_temp().unwrap(), None);
    let old = TempSize {
        ts_ms: DAY,
        folders: 213_879,
        files: 80_958,
        capped: true,
    };
    let new = TempSize {
        ts_ms: 2 * DAY,
        folders: 3_558,
        files: 3_547,
        capped: false,
    };
    store.put_temp(&new).unwrap();
    store.put_temp(&old).unwrap();
    assert_eq!(store.last_temp().unwrap(), Some(new));
}

#[test]
fn boots_and_temp_counts_follow_the_records_retention() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store.put_boot(&boot(DAY)).unwrap();
    store.put_boot(&boot(400 * DAY)).unwrap();
    store
        .put_temp(&TempSize {
            ts_ms: DAY,
            ..TempSize::default()
        })
        .unwrap();
    store
        .put_temp(&TempSize {
            ts_ms: 400 * DAY,
            ..TempSize::default()
        })
        .unwrap();
    let report = store.prune(401 * DAY, &RETENTION).unwrap();
    assert_eq!((report.boots, report.temp_readings), (1, 1));
    assert_eq!(store.boots(10).unwrap().len(), 1);
}

#[test]
fn clearing_the_history_clears_the_boots() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store.put_boot(&boot(DAY)).unwrap();
    store
        .put_temp(&TempSize {
            ts_ms: DAY,
            ..TempSize::default()
        })
        .unwrap();
    store.clear_history().unwrap();
    assert!(store.boots(10).unwrap().is_empty());
    assert_eq!(store.last_temp().unwrap(), None);
}
