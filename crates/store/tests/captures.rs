use vestigium_core::{
    AppFrames, CaptureSource, DeepCapture, DriverLatency, LongRoutine, Retention, RoutineKind,
    RoutineStats,
};
use vestigium_store::{CaptureSummary, SCHEMA_VERSION, Store};

const DAY_MS: i64 = 86_400_000;

fn capture(start_ms: i64, end_ms: i64) -> DeepCapture {
    let mut dpc = RoutineStats::default();
    dpc.add(40);
    dpc.add(1_500);
    DeepCapture {
        start_ms,
        end_ms,
        kernel: CaptureSource::Ok,
        frames: CaptureSource::Unavailable("not measured".into()),
        lost_events: 0,
        drivers: vec![DriverLatency {
            module: "example.sys".into(),
            dpc,
            isr: RoutineStats::default(),
        }],
        longest: vec![LongRoutine {
            ts_ms: start_ms + 10,
            core: 0,
            module: "example.sys".into(),
            kind: RoutineKind::Dpc,
            us: 1_500,
        }],
        apps: vec![AppFrames {
            name: "game.exe".into(),
            pid: 7,
            frames: 600,
            median_ms: 16.7,
            p99_ms: 33.0,
            worst_ms: 120.0,
            hitches: 3,
        }],
        hitches: Vec::new(),
        hiccups: Vec::new(),
    }
}

#[test]
fn capture_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    let saved = capture(1_000, 301_000);

    let id = store.insert_capture(&saved).unwrap();

    assert_eq!(store.capture(id).unwrap(), Some(saved));
    assert_eq!(store.capture(id + 1).unwrap(), None);
    assert_eq!(
        store.captures(0, 2_000).unwrap(),
        vec![CaptureSummary {
            id,
            start_ms: 1_000,
            end_ms: 301_000
        }]
    );
    assert!(store.captures(400_000, 500_000).unwrap().is_empty());
}

#[test]
fn a_capture_of_the_first_format_still_opens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    let id = {
        let mut store = Store::open(&path).unwrap();
        store.insert_capture(&capture(1_000, 2_000)).unwrap()
    };
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute("UPDATE capture SET version = 1", []).unwrap();
    drop(raw);
    let store = Store::open(&path).unwrap();

    assert_eq!(store.capture(id).unwrap(), Some(capture(1_000, 2_000)));

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute("UPDATE capture SET version = 99", []).unwrap();
    drop(raw);
    assert!(store.capture(id).is_err());
}

#[test]
fn migration_from_v2_keeps_everything() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "CREATE TABLE gap (id INTEGER PRIMARY KEY, start_ms INTEGER NOT NULL,
                 end_ms INTEGER NOT NULL, reason INTEGER NOT NULL);
             INSERT INTO gap (start_ms, end_ms, reason) VALUES (10, 20, 1);
             CREATE TABLE event (id INTEGER PRIMARY KEY, provider TEXT NOT NULL,
                 meaning INTEGER NOT NULL);
             CREATE TABLE segment (id INTEGER PRIMARY KEY, series INTEGER NOT NULL,
                 start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, data BLOB NOT NULL);
             PRAGMA user_version = 2;",
        )
        .unwrap();
    }

    let mut store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.gaps(0, 100).unwrap().len(), 1);
    let id = store.insert_capture(&capture(1, 2)).unwrap();
    assert!(store.capture(id).unwrap().is_some());
}

#[test]
fn prune_keeps_captures_inside_retention() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    let now = 1_000 * DAY_MS;
    let cutoff = now - 365 * DAY_MS;
    store
        .insert_capture(&capture(cutoff - 10, cutoff - 1))
        .unwrap();
    store
        .insert_capture(&capture(cutoff - 9, cutoff + 1))
        .unwrap();

    let report = store.prune(now, &Retention::default()).unwrap();

    assert_eq!(report.captures, 1);
    assert_eq!(store.captures(i64::MIN, i64::MAX).unwrap().len(), 1);
}

#[test]
fn clear_history_deletes_captures() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    store.insert_capture(&capture(1, 2)).unwrap();

    store.clear_history().unwrap();

    assert!(store.captures(i64::MIN, i64::MAX).unwrap().is_empty());
}

#[test]
fn the_same_capture_stored_twice_is_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();

    let first = store.insert_capture(&capture(1_000, 2_000)).unwrap();
    let again = store.insert_capture(&capture(1_000, 2_000)).unwrap();
    let other = store.insert_capture(&capture(5_000, 6_000)).unwrap();

    assert_eq!(again, first);
    assert_ne!(other, first);
    assert_eq!(store.captures(i64::MIN, i64::MAX).unwrap().len(), 2);
}
