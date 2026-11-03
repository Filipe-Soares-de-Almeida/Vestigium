use vestigium_core::{
    Applier, Hive, RestorePoint, Retention, Setting, SettingChange, SettingResult, TweakChange,
    TweakId, Value, undo,
};
use vestigium_store::{SCHEMA_VERSION, Store};

const DAY: i64 = 24 * 60 * 60_000;

fn store(dir: &tempfile::TempDir) -> Store {
    Store::open(&dir.path().join("vestigium.db")).unwrap()
}

fn mpo_change(from: Value, to: Value) -> Vec<SettingChange> {
    vec![SettingChange {
        setting: Setting::Dword {
            hive: Hive::LocalMachine,
            key: r"SOFTWARE\Microsoft\Windows\Dwm".into(),
            name: "OverlayTestMode".into(),
        },
        from,
        to,
    }]
}

fn change(tweak: TweakId, changes: Vec<SettingChange>, undo_of: Option<i64>) -> TweakChange {
    TweakChange {
        tweak,
        subject: None,
        undo_of,
        results: changes
            .into_iter()
            .map(|change| SettingResult {
                change,
                error: None,
            })
            .collect(),
        restore_point: Some(RestorePoint::Reused {
            at_ms: Some(DAY / 2),
        }),
    }
}

#[test]
fn migrates_v5_to_v6() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "CREATE TABLE gap (id INTEGER PRIMARY KEY, start_ms INTEGER NOT NULL,
                 end_ms INTEGER NOT NULL, reason INTEGER NOT NULL);
             INSERT INTO gap (start_ms, end_ms, reason) VALUES (10, 20, 1);
             PRAGMA user_version = 5;",
        )
        .unwrap();
    }

    let mut store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.gaps(0, 100).unwrap().len(), 1);
    let applied = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );
    store
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();
    assert_eq!(store.tweaks_in_place().unwrap().len(), 1);
}

#[test]
fn undo_keeps_the_original_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let forward = mpo_change(Value::Absent, Value::Dword(5));
    let applied = change(TweakId::Mpo, forward.clone(), None);
    let id = store
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();

    let back = change(TweakId::Mpo, undo(&forward), Some(id));
    store
        .record_tweak_change(&back, 3 * DAY, Applier::Elevated)
        .unwrap();

    let rows = store.tweak_changes(0, 10 * DAY).unwrap();
    assert_eq!(rows.len(), 2);
    let original = rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(original.undone_ms, Some(3 * DAY));
    assert_eq!(original.change, applied);
    assert_eq!(original.applier, Applier::Elevated);
    assert!(store.tweaks_in_place().unwrap().is_empty());
}

#[test]
fn an_absent_value_survives_the_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let service = Value::Service {
        start: 2,
        delayed: true,
        running: true,
    };
    let applied = TweakChange {
        tweak: TweakId::WindowsSearch,
        subject: None,
        undo_of: None,
        results: vec![SettingResult {
            change: SettingChange {
                setting: Setting::Service {
                    name: "WSearch".into(),
                },
                from: service,
                to: Value::Service {
                    start: 4,
                    delayed: false,
                    running: false,
                },
            },
            error: Some(5),
        }],
        restore_point: None,
    };
    store
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();
    let mpo = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );
    store
        .record_tweak_change(&mpo, 2 * DAY, Applier::Elevated)
        .unwrap();

    let rows = store.tweaks_in_place().unwrap();
    assert_eq!(rows[0].change.results[0].change.from, Value::Absent);
    assert_eq!(rows[1].change, applied);
    assert!(!rows[1].change.succeeded());
}

#[test]
fn prune_keeps_changes_still_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let old = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );
    let kept = store
        .record_tweak_change(&old, DAY, Applier::Elevated)
        .unwrap();
    let gone = change(TweakId::GameDvr, Vec::new(), None);
    let undone = store
        .record_tweak_change(&gone, DAY, Applier::User)
        .unwrap();
    store
        .record_tweak_change(
            &change(TweakId::GameDvr, Vec::new(), Some(undone)),
            2 * DAY,
            Applier::User,
        )
        .unwrap();

    let report = store.prune(800 * DAY, &Retention::default()).unwrap();

    assert_eq!(report.tweak_changes, 2);
    let left = store.tweak_changes(0, 900 * DAY).unwrap();
    assert_eq!(left.iter().map(|r| r.id).collect::<Vec<_>>(), vec![kept]);
}
