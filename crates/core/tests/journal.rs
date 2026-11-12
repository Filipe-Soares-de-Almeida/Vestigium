use std::cell::RefCell;

use vestigium_core::{
    ERROR_CHANGED_SINCE, ERROR_INTERRUPTED_WRITTEN, ERROR_MAY_HAVE_CHANGED, ERROR_UNREADABLE,
    IntentSetting, JournalEntry, NoJournal, PlanItem, Setting, SettingChange, SettingResult,
    TweakChange, TweakId, TweakJournal, Value, backed_by_journal, check_undo, undo,
};

/// A journal held in memory; `fails` makes every lookup an error.
#[derive(Default)]
struct Memory {
    entries: Vec<JournalEntry>,
    fails: bool,
    asked: RefCell<Vec<u64>>,
}

impl TweakJournal for Memory {
    fn entry(&self, id: u64) -> Result<Option<JournalEntry>, String> {
        self.asked.borrow_mut().push(id);
        if self.fails {
            return Err("unreadable".into());
        }
        let at = usize::try_from(id).ok().and_then(|id| id.checked_sub(1));
        Ok(at.and_then(|at| self.entries.get(at)).cloned())
    }

    fn record(&mut self, _: &JournalEntry) -> Result<u64, String> {
        Err("read only".into())
    }
}

fn count() -> Setting {
    TweakId::AppDumps.settings()[2].clone()
}

fn entry(tweak: TweakId, setting: Setting, before: Value, to: Value) -> JournalEntry {
    JournalEntry {
        ts_ms: 1,
        tweak,
        subject: None,
        undoes: None,
        settings: vec![IntentSetting {
            setting,
            before: Some(before),
            to,
        }],
    }
}

/// What the helper wrote for the AppDumps tweak: DumpCount from 10 to 25.
fn written_change(journal: Option<u64>) -> TweakChange {
    TweakChange {
        tweak: TweakId::AppDumps,
        subject: None,
        undo_of: None,
        results: vec![SettingResult {
            change: SettingChange {
                setting: count(),
                from: Value::Dword(10),
                to: Value::Dword(25),
            },
            error: None,
        }],
        restore_point: None,
        journal,
    }
}

fn dumps_journal() -> Memory {
    Memory {
        entries: vec![entry(
            TweakId::AppDumps,
            count(),
            Value::Dword(10),
            Value::Dword(25),
        )],
        ..Memory::default()
    }
}

fn undo_of_written(journal: Option<u64>) -> PlanItem {
    PlanItem {
        tweak: TweakId::AppDumps,
        subject: None,
        undo_of: Some(4),
        undo_journal: journal,
        changes: undo(&written_change(Some(1)).written()),
    }
}

#[test]
fn an_undo_that_matches_its_entry_is_allowed() {
    assert_eq!(
        check_undo(&undo_of_written(Some(1)), &dumps_journal()),
        Ok(())
    );
}

#[test]
fn an_undo_that_names_no_entry_is_left_to_the_other_checks() {
    let journal = Memory::default();
    assert_eq!(check_undo(&undo_of_written(None), &journal), Ok(()));
    assert!(
        journal.asked.borrow().is_empty(),
        "the journal is not asked"
    );
    let plain = PlanItem {
        undo_of: None,
        undo_journal: Some(77),
        ..undo_of_written(None)
    };
    assert_eq!(check_undo(&plain, &journal), Ok(()), "not an undo");
}

#[test]
fn an_undo_of_a_change_the_journal_does_not_hold_is_refused() {
    assert!(check_undo(&undo_of_written(Some(2)), &dumps_journal()).is_err());
    assert!(check_undo(&undo_of_written(Some(1)), &NoJournal).is_err());
    let unreadable = Memory {
        fails: true,
        ..dumps_journal()
    };
    assert!(check_undo(&undo_of_written(Some(1)), &unreadable).is_err());
}

#[test]
fn an_undo_that_puts_back_other_values_than_the_entry_held_is_refused() {
    let mut forged = undo_of_written(Some(1));
    forged.changes[0].to = Value::Dword(u32::MAX);
    assert!(check_undo(&forged, &dumps_journal()).is_err());

    let mut from_elsewhere = undo_of_written(Some(1));
    from_elsewhere.changes[0].from = Value::Dword(26);
    assert!(check_undo(&from_elsewhere, &dumps_journal()).is_err());
}

#[test]
fn an_undo_cannot_reach_a_setting_the_entry_did_not_touch() {
    let mut wider = undo_of_written(Some(1));
    wider.changes.push(SettingChange {
        setting: TweakId::AppDumps.settings()[1].clone(),
        from: Value::Dword(2),
        to: Value::Dword(0),
    });
    assert!(check_undo(&wider, &dumps_journal()).is_err());
}

#[test]
fn an_undo_must_be_of_the_same_tweak_and_subject_and_not_of_an_undo() {
    let mut other_tweak = undo_of_written(Some(1));
    other_tweak.tweak = TweakId::CrashDump;
    assert!(check_undo(&other_tweak, &dumps_journal()).is_err());

    let mut subject = undo_of_written(Some(1));
    subject.subject = Some("x".into());
    assert!(check_undo(&subject, &dumps_journal()).is_err());

    let mut of_an_undo = dumps_journal();
    of_an_undo.entries[0].undoes = Some(3);
    assert!(check_undo(&undo_of_written(Some(1)), &of_an_undo).is_err());
}

#[test]
fn an_undo_of_an_entry_that_could_not_read_the_old_value_is_refused() {
    let mut blind = dumps_journal();
    blind.entries[0].settings[0].before = None;
    assert!(check_undo(&undo_of_written(Some(1)), &blind).is_err());
}

#[test]
fn a_service_running_flag_does_not_decide_an_undo() {
    let service = Setting::Service {
        name: "SysMain".into(),
    };
    let value = |start, running| Value::Service {
        start,
        delayed: false,
        running,
    };
    let journal = Memory {
        entries: vec![entry(
            TweakId::SysMain,
            service.clone(),
            value(2, true),
            value(4, false),
        )],
        ..Memory::default()
    };
    let item = PlanItem {
        tweak: TweakId::SysMain,
        subject: None,
        undo_of: Some(1),
        undo_journal: Some(1),
        changes: vec![SettingChange {
            setting: service,
            from: value(4, true),
            to: value(2, false),
        }],
    };
    assert_eq!(check_undo(&item, &journal), Ok(()));
}

#[test]
fn a_change_the_journal_describes_is_kept() {
    assert_eq!(
        backed_by_journal(&written_change(Some(1)), &dumps_journal()),
        Ok(true)
    );
}

#[test]
fn a_change_with_no_entry_that_wrote_something_is_not_kept() {
    let forged = written_change(None);
    assert_eq!(backed_by_journal(&forged, &dumps_journal()), Ok(false));
    let mut undoing = written_change(None);
    undoing.undo_of = Some(1);
    undoing.results.clear();
    assert_eq!(backed_by_journal(&undoing, &dumps_journal()), Ok(false));
}

#[test]
fn a_change_that_does_not_match_its_entry_is_not_kept() {
    let missing = written_change(Some(5));
    assert_eq!(backed_by_journal(&missing, &dumps_journal()), Ok(false));

    let mut other_value = written_change(Some(1));
    other_value.results[0].change.to = Value::Dword(26);
    assert_eq!(backed_by_journal(&other_value, &dumps_journal()), Ok(false));

    let mut other_setting = written_change(Some(1));
    other_setting.results[0].change.setting = TweakId::AppDumps.settings()[1].clone();
    assert_eq!(
        backed_by_journal(&other_setting, &dumps_journal()),
        Ok(false)
    );

    let mut other_tweak = written_change(Some(1));
    other_tweak.tweak = TweakId::CrashDump;
    assert_eq!(backed_by_journal(&other_tweak, &dumps_journal()), Ok(false));

    let mut claims_undo = written_change(Some(1));
    claims_undo.undo_of = Some(2);
    assert_eq!(backed_by_journal(&claims_undo, &dumps_journal()), Ok(false));
}

#[test]
fn a_change_that_moved_nothing_needs_no_entry() {
    for code in [5, ERROR_CHANGED_SINCE, ERROR_UNREADABLE] {
        let mut refused = written_change(None);
        refused.results[0].error = Some(code);
        assert_eq!(
            backed_by_journal(&refused, &Memory::default()),
            Ok(true),
            "{code}"
        );
    }
    for code in [ERROR_MAY_HAVE_CHANGED, ERROR_INTERRUPTED_WRITTEN] {
        let mut maybe = written_change(None);
        maybe.results[0].error = Some(code);
        assert_eq!(
            backed_by_journal(&maybe, &Memory::default()),
            Ok(false),
            "{code}"
        );
    }
}

#[test]
fn an_unreadable_journal_is_an_error_not_a_verdict() {
    let unreadable = Memory {
        fails: true,
        ..dumps_journal()
    };
    assert!(backed_by_journal(&written_change(Some(1)), &unreadable).is_err());
}
