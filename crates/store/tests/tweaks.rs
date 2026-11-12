use vestigium_core::{
    Applier, Hive, MachineInfo, RestorePoint, Retention, Setting, SettingChange, SettingResult,
    Settings, TWEAKS, ThemePref, TweakChange, TweakId, Value, undo,
};
use vestigium_store::{CarryOver, SCHEMA_VERSION, Store};

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
        journal: None,
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
fn a_refused_undo_leaves_the_change_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let forward = mpo_change(Value::Absent, Value::Dword(5));
    let applied = change(TweakId::Mpo, forward.clone(), None);
    let id = store
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();

    let mut refused = change(TweakId::Mpo, undo(&forward), Some(id));
    refused.results[0].error = Some(vestigium_core::ERROR_CHANGED_SINCE);
    store
        .record_tweak_change(&refused, 3 * DAY, Applier::Elevated)
        .unwrap();

    assert_eq!(store.tweaks_in_place().unwrap().len(), 1);
    let original = store.tweak_change(id).unwrap().unwrap();
    assert_eq!(original.undone_ms, None);
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
        journal: None,
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

#[test]
fn a_change_stored_before_the_scheme_was_kept_still_reads() {
    use vestigium_core::{
        PCIE_ASPM, SUB_PCIEXPRESS, SettingChangeV1, SettingResultV1, SettingV1, TweakChangeV1,
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());
    let old = TweakChangeV1 {
        tweak: TweakId::PcieAspm,
        subject: None,
        undo_of: None,
        results: vec![SettingResultV1 {
            change: SettingChangeV1 {
                setting: SettingV1::PowerAc {
                    subgroup: SUB_PCIEXPRESS,
                    setting: PCIE_ASPM,
                },
                from: Value::Index(2),
                to: Value::Index(0),
            },
            error: None,
        }],
        restore_point: None,
    };
    let mut blob = vec![1u8];
    blob.extend(postcard::to_stdvec(&vec![old]).unwrap());
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO tweak_change (version, tweak, subject, applied_ms, undone_ms, undo_of,
                 elevated, data) VALUES (1, ?1, NULL, ?2, NULL, NULL, 0, ?3)",
            rusqlite::params![
                TWEAKS.iter().position(|t| *t == TweakId::PcieAspm).unwrap() as i64,
                DAY,
                blob
            ],
        )
        .unwrap();

    let rows = Store::open(&path).unwrap().tweaks_in_place().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].change.results[0].change.setting,
        Setting::PowerAc {
            subgroup: SUB_PCIEXPRESS,
            setting: PCIE_ASPM,
            scheme: None
        }
    );
    assert_eq!(rows[0].change.results[0].change.to, Value::Index(0));
}

#[test]
fn the_same_change_recorded_twice_is_one_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let forward = mpo_change(Value::Absent, Value::Dword(5));
    let applied = change(TweakId::Mpo, forward.clone(), None);

    let first = store
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();
    let again = store
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();
    let later = store
        .record_tweak_change(&applied, 2 * DAY, Applier::Elevated)
        .unwrap();
    let back = change(TweakId::Mpo, undo(&forward), Some(first));
    let undone = store
        .record_tweak_change(&back, 3 * DAY, Applier::Elevated)
        .unwrap();
    let undone_again = store
        .record_tweak_change(&back, 3 * DAY, Applier::Elevated)
        .unwrap();

    assert_eq!(again, first);
    assert_ne!(later, first);
    assert_eq!(undone_again, undone);
    assert_eq!(store.tweak_changes(0, 10 * DAY).unwrap().len(), 3);
}

#[test]
fn changes_of_one_tweak_for_different_subjects_at_one_time_are_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let mut one = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );
    one.subject = Some("one".into());
    let mut two = one.clone();
    two.subject = Some("two".into());
    let none = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );

    let ids: Vec<i64> = [&one, &two, &none]
        .iter()
        .map(|c| {
            store
                .record_tweak_change(c, DAY, Applier::Elevated)
                .unwrap()
        })
        .collect();

    assert_eq!(ids.len(), 3);
    assert_eq!(store.tweak_changes(0, 10 * DAY).unwrap().len(), 3);
}

#[test]
fn an_undo_found_written_after_an_interrupted_helper_marks_the_change_undone() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let forward = mpo_change(Value::Absent, Value::Dword(5));
    let id = store
        .record_tweak_change(
            &change(TweakId::Mpo, forward.clone(), None),
            DAY,
            Applier::Elevated,
        )
        .unwrap();
    let mut back = change(TweakId::Mpo, undo(&forward), Some(id));
    back.results[0].error = Some(vestigium_core::ERROR_INTERRUPTED_WRITTEN);

    store
        .record_tweak_change(&back, 3 * DAY, Applier::Elevated)
        .unwrap();

    assert!(store.tweaks_in_place().unwrap().is_empty());
}

#[test]
fn every_tweak_is_stored_under_its_own_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    for (tweak, n) in TWEAKS.into_iter().zip(1..) {
        let made = change(tweak, mpo_change(Value::Absent, Value::Dword(0)), None);
        store
            .record_tweak_change(&made, n * 1_000, Applier::User)
            .unwrap();
    }

    let found: Vec<TweakId> = store
        .tweak_changes(0, i64::MAX)
        .unwrap()
        .into_iter()
        .rev()
        .map(|row| row.change.tweak)
        .collect();
    let raw = rusqlite::Connection::open(dir.path().join("vestigium.db")).unwrap();
    let codes: Vec<i64> = raw
        .prepare("SELECT tweak FROM tweak_change ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(found, TWEAKS);
    assert_eq!(codes, (0..13).collect::<Vec<i64>>());
}

fn aside_pair(dir: &tempfile::TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    (dir.path().join("old.db"), dir.path().join("vestigium.db"))
}

/// A history with one tweak still in place, a theme and a machine, closed cleanly.
fn history_with_tweak(path: &std::path::Path) -> i64 {
    let mut old = Store::open(path).unwrap();
    let applied = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );
    let id = old
        .record_tweak_change(&applied, DAY, Applier::Elevated)
        .unwrap();
    old.set_settings(&Settings {
        theme: ThemePref::Dark,
        ..Settings::default()
    })
    .unwrap();
    old.set_machine(&MachineInfo {
        logical_cpus: 6,
        mem_total_bytes: 8 << 30,
        disks: Vec::new(),
    })
    .unwrap();
    id
}

#[test]
fn a_new_database_takes_over_tweaks_settings_and_machine() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = aside_pair(&dir);
    let id = history_with_tweak(&old);
    let before = std::fs::read(&old).unwrap();
    let mut store = Store::open(&new).unwrap();

    let carried = store.carry_over_from(&old);

    assert_eq!(
        carried,
        CarryOver {
            tweaks: true,
            settings: true,
            machine: true
        }
    );
    let kept = store.tweaks_in_place().unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].id, id);
    assert_eq!(store.settings().unwrap().theme, ThemePref::Dark);
    assert_eq!(store.machine().unwrap().unwrap().logical_cpus, 6);
    assert_eq!(std::fs::read(&old).unwrap(), before);
}

#[test]
fn what_the_new_database_already_holds_is_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = aside_pair(&dir);
    history_with_tweak(&old);
    let mut store = Store::open(&new).unwrap();
    store
        .set_settings(&Settings {
            theme: ThemePref::Light,
            ..Settings::default()
        })
        .unwrap();

    store.carry_over_from(&old);
    store.carry_over_from(&old);

    assert_eq!(store.settings().unwrap().theme, ThemePref::Light);
    assert_eq!(store.tweaks_in_place().unwrap().len(), 1);
}

#[test]
fn a_table_the_old_file_cannot_give_does_not_stop_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = aside_pair(&dir);
    {
        let raw = rusqlite::Connection::open(&old).unwrap();
        raw.execute_batch(
            "CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO setting VALUES ('theme', '\"Dark\"');",
        )
        .unwrap();
    }
    let mut store = Store::open(&new).unwrap();

    let carried = store.carry_over_from(&old);

    assert_eq!(
        carried,
        CarryOver {
            tweaks: false,
            settings: true,
            machine: false
        }
    );
    assert_eq!(store.settings().unwrap().theme, ThemePref::Dark);
    assert!(store.tweaks_in_place().unwrap().is_empty());
}

#[test]
fn an_unreadable_old_file_gives_nothing_and_stays_as_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = aside_pair(&dir);
    let bytes: Vec<u8> = (0..4096u32).map(|n| (n * 7 + 3) as u8).collect();
    std::fs::write(&old, &bytes).unwrap();
    let wal = dir.path().join("old.db-wal");
    std::fs::write(&wal, b"wal bytes").unwrap();
    let mut store = Store::open(&new).unwrap();

    let carried = store.carry_over_from(&old);

    assert_eq!(carried, CarryOver::default());
    assert!(store.tweaks_in_place().unwrap().is_empty());
    assert!(store.machine().unwrap().is_none());
    assert_eq!(std::fs::read(&old).unwrap(), bytes);
    assert_eq!(std::fs::read(&wal).unwrap(), b"wal bytes");
}

#[test]
fn a_missing_old_file_is_not_created() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = aside_pair(&dir);
    let mut store = Store::open(&new).unwrap();

    store.carry_over_from(&old);

    assert!(!old.exists());
}

#[test]
fn a_path_with_special_characters_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let odd = dir.path().join("a b#c%d.db");
    let new = dir.path().join("vestigium.db");
    let id = history_with_tweak(&odd);
    let mut store = Store::open(&new).unwrap();

    store.carry_over_from(&odd);

    assert_eq!(store.tweaks_in_place().unwrap()[0].id, id);
}

#[test]
fn the_journal_entry_of_a_change_survives_the_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = store(&dir);
    let mut mpo = change(
        TweakId::Mpo,
        mpo_change(Value::Absent, Value::Dword(5)),
        None,
    );
    mpo.journal = Some(41);
    let id = store
        .record_tweak_change(&mpo, DAY, Applier::Elevated)
        .unwrap();

    assert_eq!(
        store.tweak_change(id).unwrap().unwrap().change.journal,
        Some(41)
    );
}

#[test]
fn a_change_stored_before_the_journal_existed_still_reads_without_an_entry() {
    use vestigium_core::TweakChangeV2;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());
    let old = TweakChangeV2 {
        tweak: TweakId::Mpo,
        subject: None,
        undo_of: None,
        results: change(
            TweakId::Mpo,
            mpo_change(Value::Absent, Value::Dword(5)),
            None,
        )
        .results,
        restore_point: None,
    };
    let mut blob = vec![1u8];
    blob.extend(postcard::to_stdvec(&vec![old]).unwrap());
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO tweak_change (version, tweak, subject, applied_ms, undone_ms, undo_of,
                 elevated, data) VALUES (2, ?1, NULL, ?2, NULL, NULL, 1, ?3)",
            rusqlite::params![
                TWEAKS.iter().position(|t| *t == TweakId::Mpo).unwrap() as i64,
                DAY,
                blob
            ],
        )
        .unwrap();

    let rows = Store::open(&path).unwrap().tweaks_in_place().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].change.journal, None);
    assert_eq!(rows[0].change.results[0].change.to, Value::Dword(5));
    assert_eq!(rows[0].applier, Applier::Elevated);
}
