use vestigium_core::Media;
use vestigium_core::Retention;
use vestigium_core::health::Subject;
use vestigium_core::hwtest::{
    BANDS, MemoryQuick, Outcome, Region, SurfaceAggregator, SurfaceScanV2, TestData, TestDataV1,
    TestDataV2, TestRun, TestRunV1, TestRunV2, VolumeCheck, VolumeCheckV1, VolumeResult,
};
use vestigium_store::{SCHEMA_VERSION, Store, encode};

const DAY: i64 = 24 * 60 * 60_000;

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(&dir.path().join("vestigium.db")).unwrap()
}

fn quick(started_ms: i64) -> TestRun {
    TestRun {
        subject: Subject::Pc,
        started_ms,
        ended_ms: started_ms + 300_000,
        outcome: Outcome::Clean,
        data: TestData::MemoryQuick(MemoryQuick {
            total_bytes: 32 << 30,
            tested_bytes: 8 << 30,
            rounds: 4,
            passes: 30,
            error_count: 0,
            errors: Vec::new(),
            failure: None,
        }),
    }
}

#[test]
fn migrates_v6_to_v7() {
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
             CREATE TABLE finding (id INTEGER PRIMARY KEY, version INTEGER NOT NULL,
                 opened_ms INTEGER NOT NULL, updated_ms INTEGER NOT NULL, closed_ms INTEGER,
                 data BLOB NOT NULL);
             CREATE TABLE capture (id INTEGER PRIMARY KEY, version INTEGER NOT NULL,
                 start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, data BLOB NOT NULL);
             CREATE TABLE scan (id INTEGER PRIMARY KEY, version INTEGER NOT NULL,
                 ts_ms INTEGER NOT NULL, data BLOB NOT NULL);
             CREATE TABLE tweak_change (id INTEGER PRIMARY KEY, version INTEGER NOT NULL,
                 tweak INTEGER NOT NULL, subject TEXT, applied_ms INTEGER NOT NULL,
                 undone_ms INTEGER, undo_of INTEGER, elevated INTEGER NOT NULL,
                 data BLOB NOT NULL);
             PRAGMA user_version = 6;",
        )
        .unwrap();
    }

    let mut store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.gaps(0, 100).unwrap().len(), 1);
    store.record_test_run(&quick(DAY)).unwrap();
    assert_eq!(store.test_runs(0, 2 * DAY).unwrap().len(), 1);
}

#[test]
fn import_twice_is_one_run() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let first = store.record_test_run(&quick(DAY)).unwrap();
    let again = store.record_test_run(&quick(DAY)).unwrap();
    let other = store.record_test_run(&quick(2 * DAY)).unwrap();
    assert_eq!(first, again);
    assert_ne!(first, other);
    let runs = store.test_runs(0, 3 * DAY).unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].run, quick(DAY));
}

#[test]
fn a_surface_map_survives_the_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let disk = 1u64 << 40;
    let mut scan = SurfaceAggregator::new(1, Media::Hdd, disk, 4096, 0);
    scan.block(5 << 30, 612.5);
    scan.unreadable(9 << 30, &[(9 << 30) + 4096]);
    let run = TestRun {
        subject: Subject::Disk("1 D:".into()),
        started_ms: DAY,
        ended_ms: DAY + 3_600_000,
        outcome: Outcome::Problems,
        data: TestData::DiskSurface(scan.finish(disk / 2, Some("stopped".into()))),
    };
    store.record_test_run(&run).unwrap();
    let back = store.test_runs(0, 2 * DAY).unwrap().pop().unwrap().run;
    assert_eq!(back, run);
    let TestData::DiskSurface(scan) = back.data else {
        panic!("a surface scan");
    };
    assert!(scan.regions.contains(&Region::Unreadable));
}

#[test]
fn runs_follow_the_records_retention_and_the_history_clear() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store.record_test_run(&quick(DAY)).unwrap();
    store.record_test_run(&quick(400 * DAY)).unwrap();
    let report = store.prune(367 * DAY, &Retention::default()).unwrap();
    assert_eq!(report.test_runs, 1);
    assert_eq!(store.test_runs(0, 500 * DAY).unwrap().len(), 1);
    store.clear_history().unwrap();
    assert!(store.test_runs(0, 500 * DAY).unwrap().is_empty());
}

#[test]
fn a_volume_check_keeps_its_dirty_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let run = TestRun {
        subject: Subject::Volume(r"D:\".into()),
        started_ms: DAY,
        ended_ms: DAY + 60_000,
        outcome: Outcome::Problems,
        data: TestData::VolumeCheck(VolumeCheck {
            volume: r"D:\".into(),
            exit_code: Some(13),
            result: VolumeResult::ProblemsFound,
            output: "text".into(),
            dirty_before: Some(false),
            dirty_after: Some(true),
        }),
    };
    store.record_test_run(&run).unwrap();
    assert_eq!(store.test_runs(0, 2 * DAY).unwrap()[0].run, run);
}

#[test]
fn a_surface_scan_stored_with_layout_version_two_still_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());
    let old = TestRunV2 {
        subject: Subject::Disk("1 D:".into()),
        started_ms: DAY,
        ended_ms: DAY + 60_000,
        outcome: Outcome::Clean,
        data: TestDataV2::DiskSurface(SurfaceScanV2 {
            disk: 1,
            media: Media::Hdd,
            disk_bytes: 1 << 30,
            sector_bytes: 512,
            from_bytes: 0,
            to_bytes: 1 << 30,
            bands: [10, 0, 0, 0, 0, 12],
            unreadable_blocks: 0,
            regions: vec![Region::Unread; 4],
            worst: Vec::new(),
            unreadable: Vec::new(),
            failure: None,
        }),
    };
    assert_eq!(BANDS, 6);
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            "INSERT INTO test_run (version, kind, subject, started_ms, ended_ms, outcome, data)
             VALUES (2, 1, 'disk:1 D:', ?1, ?2, 0, ?3)",
            rusqlite::params![
                old.started_ms,
                old.ended_ms,
                encode(std::slice::from_ref(&old))
            ],
        )
        .unwrap();
    }
    let store = Store::open(&path).unwrap();
    let runs = store.test_runs(0, 2 * DAY).unwrap();
    assert_eq!(runs.len(), 1);
    let TestData::DiskSurface(scan) = &runs[0].run.data else {
        panic!("a surface scan");
    };
    assert_eq!(scan.slow_blocks(), 12);
    assert!(!scan.outside_io);
}

#[test]
fn a_run_stored_with_layout_version_one_still_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());
    let old = TestRunV1 {
        subject: Subject::Volume(r"D:\".into()),
        started_ms: DAY,
        ended_ms: DAY + 60_000,
        outcome: Outcome::Problems,
        data: TestDataV1::VolumeCheck(VolumeCheckV1 {
            volume: r"D:\".into(),
            exit_code: Some(13),
            result: VolumeResult::ProblemsFound,
            output: "text".into(),
        }),
    };
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            "INSERT INTO test_run (version, kind, subject, started_ms, ended_ms, outcome, data)
             VALUES (1, 2, 'volume:D:\', ?1, ?2, 1, ?3)",
            rusqlite::params![
                old.started_ms,
                old.ended_ms,
                encode(std::slice::from_ref(&old))
            ],
        )
        .unwrap();
    }
    let store = Store::open(&path).unwrap();
    let runs = store.test_runs(0, 2 * DAY).unwrap();
    assert_eq!(runs.len(), 1);
    let TestData::VolumeCheck(check) = &runs[0].run.data else {
        panic!("a volume check");
    };
    assert_eq!(check.exit_code, Some(13));
    assert_eq!((check.dirty_before, check.dirty_after), (None, None));
}

#[test]
fn every_outcome_round_trips_under_its_own_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let runs: Vec<TestRun> = Outcome::ALL
        .into_iter()
        .zip(1..)
        .map(|(outcome, n)| TestRun {
            outcome,
            ..quick(n * DAY)
        })
        .collect();
    for run in &runs {
        store.record_test_run(run).unwrap();
    }

    let found: Vec<TestRun> = store
        .test_runs(0, 10 * DAY)
        .unwrap()
        .into_iter()
        .map(|row| row.run)
        .collect();
    let raw = rusqlite::Connection::open(dir.path().join("vestigium.db")).unwrap();
    let codes: Vec<i64> = raw
        .prepare("SELECT outcome FROM test_run ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(found, runs);
    assert_eq!(codes, [0, 1, 2, 3]);
}
