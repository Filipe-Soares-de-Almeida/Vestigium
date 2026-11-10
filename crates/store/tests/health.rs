use vestigium_core::Retention;
use vestigium_core::health::{
    BootReport, BootTime, Concern, DiskHealth, Finding, FindingChange, FindingKind, HealthFact,
    Measure, Reference, ScanResult, SmartAttribute, Subject,
};
use vestigium_store::{SCHEMA_VERSION, Store};

const DAY: i64 = 24 * 60 * 60_000;

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(&dir.path().join("vestigium.db")).unwrap()
}

fn finding(kind: FindingKind, days: u32) -> Finding {
    Finding {
        kind,
        subject: Subject::Pc,
        concern: Concern::Attention,
        value: Measure::Days(days),
        reference: Reference::AtMost(Measure::Days(1)),
        evidence: vec![HealthFact::MemoryHighDays { days }],
        next_steps: Vec::new(),
    }
}

fn scan(ts_ms: i64, boot_ms: u32) -> ScanResult {
    ScanResult {
        ts_ms,
        boot: Some(BootReport {
            boots: vec![BootTime {
                ts_ms,
                boot_ms,
                main_path_ms: boot_ms / 2,
                post_boot_ms: boot_ms / 2,
            }],
            delays: Vec::new(),
        }),
    }
}

#[test]
fn findings_open_update_and_close() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);

    store
        .apply_findings(
            &[
                FindingChange::Open(finding(FindingKind::MemoryOften, 3)),
                FindingChange::Open(finding(FindingKind::Throttling, 1)),
            ],
            10 * DAY,
        )
        .unwrap();
    let open = store.open_findings().unwrap();
    assert_eq!(open.len(), 2);
    assert_eq!(open[0].opened_ms, 10 * DAY);

    store
        .apply_findings(
            &[
                FindingChange::Update {
                    id: open[0].id,
                    finding: finding(FindingKind::MemoryOften, 5),
                },
                FindingChange::Close { id: open[1].id },
            ],
            12 * DAY,
        )
        .unwrap();

    let open = store.open_findings().unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].finding.value, Measure::Days(5));
    assert_eq!(
        open[0].opened_ms,
        10 * DAY,
        "an update keeps when it opened"
    );

    let history = store.findings(0, 20 * DAY).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].updated_ms, 12 * DAY);
    assert_eq!(
        history[0].first.as_ref().map(|f| &f.value),
        Some(&Measure::Days(3)),
        "an update keeps what it measured when it opened"
    );
    assert_eq!(history[1].closed_ms, Some(12 * DAY));
    assert_eq!(
        store.findings(13 * DAY, 20 * DAY).unwrap().len(),
        1,
        "a finding closed before the span is left out"
    );
    assert!(store.findings(0, 9 * DAY).unwrap().is_empty());
}

#[test]
fn a_closed_finding_is_not_changed_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store
        .apply_findings(
            &[FindingChange::Open(finding(FindingKind::Throttling, 1))],
            DAY,
        )
        .unwrap();
    let id = store.open_findings().unwrap()[0].id;
    store
        .apply_findings(&[FindingChange::Close { id }], 2 * DAY)
        .unwrap();

    store
        .apply_findings(
            &[
                FindingChange::Close { id },
                FindingChange::Update {
                    id,
                    finding: finding(FindingKind::Throttling, 9),
                },
            ],
            3 * DAY,
        )
        .unwrap();

    let record = &store.findings(0, 10 * DAY).unwrap()[0];
    assert_eq!(record.closed_ms, Some(2 * DAY));
    assert_eq!(record.finding.value, Measure::Days(1));
}

#[test]
fn open_findings_survive_prune() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store
        .apply_findings(
            &[
                FindingChange::Open(finding(FindingKind::MemoryOften, 3)),
                FindingChange::Open(finding(FindingKind::Throttling, 1)),
            ],
            DAY,
        )
        .unwrap();
    let closing = store.open_findings().unwrap()[1].id;
    store
        .apply_findings(&[FindingChange::Close { id: closing }], 2 * DAY)
        .unwrap();

    let report = store.prune(400 * DAY, &Retention::default()).unwrap();

    assert_eq!(report.findings, 1);
    let left = store.open_findings().unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].finding.kind, FindingKind::MemoryOften);
}

#[test]
fn last_scan_is_the_newest_and_survives_prune() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    assert_eq!(store.last_scan().unwrap(), None);
    store.insert_scan(&scan(DAY, 90_000)).unwrap();
    store.insert_scan(&scan(3 * DAY, 60_000)).unwrap();
    store.insert_scan(&scan(2 * DAY, 70_000)).unwrap();

    assert_eq!(store.last_scan().unwrap(), Some(scan(3 * DAY, 60_000)));

    let report = store.prune(800 * DAY, &Retention::default()).unwrap();
    assert_eq!(report.scans, 2, "the newest scan is kept, however old");
    assert_eq!(store.last_scan().unwrap(), Some(scan(3 * DAY, 60_000)));
}

#[test]
fn clear_history_deletes_findings_and_scans() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store
        .apply_findings(
            &[FindingChange::Open(finding(FindingKind::MemoryOften, 3))],
            DAY,
        )
        .unwrap();
    store.insert_scan(&scan(DAY, 90_000)).unwrap();

    store.clear_history().unwrap();

    assert!(store.open_findings().unwrap().is_empty());
    assert_eq!(store.last_scan().unwrap(), None);
}

#[test]
fn migration_from_v3_adds_findings_and_scans() {
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
             CREATE TABLE capture (id INTEGER PRIMARY KEY, version INTEGER NOT NULL,
                 start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, data BLOB NOT NULL);
             PRAGMA user_version = 3;",
        )
        .unwrap();
    }

    let mut store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.gaps(0, 100).unwrap().len(), 1);
    store
        .apply_findings(
            &[FindingChange::Open(finding(FindingKind::MemoryOften, 3))],
            DAY,
        )
        .unwrap();
    store.insert_scan(&scan(DAY, 90_000)).unwrap();
    assert_eq!(store.open_findings().unwrap().len(), 1);
}

fn disk_reading(read_ms: i64, raw: u64) -> DiskHealth {
    DiskHealth {
        name: "0 C:".to_string(),
        read_ms,
        predict_failure: false,
        attributes: vec![
            SmartAttribute {
                id: 5,
                current: 100,
                worst: 100,
                raw,
            },
            SmartAttribute {
                id: 194,
                current: 64,
                worst: 51,
                raw: 36,
            },
        ],
        nvme: None,
    }
}

#[test]
fn smart_keeps_one_reading_a_day_per_attribute() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);

    store
        .record_smart(&disk_reading(10 * DAY + 1_000, 2))
        .unwrap();
    store
        .record_smart(&disk_reading(10 * DAY + 5_000, 3))
        .unwrap();
    store.record_smart(&disk_reading(11 * DAY, 4)).unwrap();

    let history = store.smart_history("0 C:", 5, 0).unwrap();
    let raws: Vec<(i64, u64)> = history.iter().map(|r| (r.read_ms, r.raw)).collect();
    assert_eq!(
        raws,
        [(10 * DAY + 5_000, 3), (11 * DAY, 4)],
        "the day's last reading"
    );
    assert_eq!(store.smart_history("0 C:", 194, 11 * DAY).unwrap().len(), 1);
    assert!(store.smart_history("1 D:", 5, 0).unwrap().is_empty());
}

#[test]
fn smart_readings_follow_the_records_retention() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store.record_smart(&disk_reading(DAY, 1)).unwrap();
    store.record_smart(&disk_reading(400 * DAY, 2)).unwrap();

    let report = store
        .prune(
            401 * DAY,
            &Retention {
                samples_hours: 48,
                minutes_days: 90,
                records_days: 365,
            },
        )
        .unwrap();

    assert_eq!(report.smart_readings, 2, "both attributes of the old day");
    assert_eq!(store.smart_history("0 C:", 5, 0).unwrap().len(), 1);
}

#[test]
fn the_same_scan_stored_twice_is_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);

    let first = store.insert_scan(&scan(DAY, 90_000)).unwrap();
    let again = store.insert_scan(&scan(DAY, 90_000)).unwrap();
    let other = store.insert_scan(&scan(2 * DAY, 80_000)).unwrap();

    assert_eq!(again, first);
    assert_ne!(other, first);
    let count: i64 = rusqlite::Connection::open(dir.path().join("vestigium.db"))
        .unwrap()
        .query_row("SELECT count(*) FROM scan", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 2);
}
