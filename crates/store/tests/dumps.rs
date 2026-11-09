use vestigium_core::{Analysis, DumpKind, DumpRecord, Fault, Retention};
use vestigium_store::{SCHEMA_VERSION, Store};

const DAY: i64 = 24 * 60 * 60_000;
const MB: u64 = 1 << 20;

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(&dir.path().join("vestigium.db")).unwrap()
}

fn record(path: &str, file_ms: i64, analysed: bool) -> DumpRecord {
    DumpRecord {
        path: path.to_string(),
        file_ms,
        kind: DumpKind::KernelMemory,
        crash_ms: file_ms - 60_000,
        size_bytes: 700 * MB,
        analysis: analysed.then(|| Analysis {
            fault: Fault::BugCheck {
                code: 0x50,
                params: [1, 2, 3, 4],
            },
            fault_module: Some("storage.sys".into()),
            stack_modules: vec!["storage.sys".into()],
            process: None,
            module_count: 150,
        }),
    }
}

const MEMORY_DMP: &str = r"C:\Windows\MEMORY.DMP";

#[test]
fn migrates_v4_to_v5() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "CREATE TABLE gap (id INTEGER PRIMARY KEY, start_ms INTEGER NOT NULL,
                 end_ms INTEGER NOT NULL, reason INTEGER NOT NULL);
             INSERT INTO gap (start_ms, end_ms, reason) VALUES (10, 20, 1);
             CREATE TABLE finding (id INTEGER PRIMARY KEY, version INTEGER NOT NULL,
                 opened_ms INTEGER NOT NULL, updated_ms INTEGER NOT NULL, closed_ms INTEGER,
                 data BLOB NOT NULL);
             PRAGMA user_version = 4;",
        )
        .unwrap();
    }

    let mut store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.gaps(0, 100).unwrap().len(), 1);
    store
        .upsert_dumps(&[record(MEMORY_DMP, DAY, true)], DAY)
        .unwrap();
    assert_eq!(store.present_dumps().unwrap().len(), 1);
}

#[test]
fn a_deleted_dump_keeps_its_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let ids = store
        .upsert_dumps(&[record(MEMORY_DMP, DAY, true)], DAY)
        .unwrap();
    assert_eq!(store.dump_space().unwrap(), 700 * MB);

    assert_eq!(store.mark_dumps_deleted(&ids, 2 * DAY).unwrap(), 1);

    let rows = store.dumps(0, 3 * DAY).unwrap();
    assert_eq!(rows[0].deleted_ms, Some(2 * DAY));
    assert!(rows[0].record.analysis.is_some());
    assert_eq!(rows[0].analyzed_ms, Some(DAY));
    assert_eq!(store.dump_space().unwrap(), 0);
    assert!(store.present_dumps().unwrap().is_empty());
}

#[test]
fn an_overwritten_memory_dump_is_a_new_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store
        .upsert_dumps(&[record(MEMORY_DMP, DAY, true)], DAY)
        .unwrap();
    store
        .upsert_dumps(&[record(MEMORY_DMP, 5 * DAY, false)], 5 * DAY)
        .unwrap();

    let rows = store.dumps(0, 6 * DAY).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].record.file_ms, 5 * DAY, "newest first");
}

#[test]
fn the_inventory_never_erases_an_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    store
        .upsert_dumps(&[record(MEMORY_DMP, DAY, true)], DAY)
        .unwrap();
    let mut seen_again = record(MEMORY_DMP, DAY, false);
    seen_again.size_bytes = 710 * MB;

    let ids = store.upsert_dumps(&[seen_again], 2 * DAY).unwrap();

    let rows = store.present_dumps().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, ids[0]);
    assert!(rows[0].record.analysis.is_some());
    assert_eq!(store.dump_space().unwrap(), 710 * MB);
}

#[test]
fn deleted_dumps_age_out_and_present_ones_stay() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let old = store
        .upsert_dumps(&[record(r"C:\Windows\Minidump\1.dmp", DAY, true)], DAY)
        .unwrap();
    store
        .upsert_dumps(&[record(MEMORY_DMP, DAY, true)], DAY)
        .unwrap();
    store.mark_dumps_deleted(&old, 2 * DAY).unwrap();

    let report = store.prune(400 * DAY, &Retention::default()).unwrap();

    assert_eq!(report.dumps, 1);
    assert_eq!(store.present_dumps().unwrap().len(), 1);
    store.clear_history().unwrap();
    assert!(store.dumps(0, 500 * DAY).unwrap().is_empty());
}
