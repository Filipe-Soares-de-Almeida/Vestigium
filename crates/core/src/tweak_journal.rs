use serde::{Deserialize, Serialize};

use crate::tweak::same_state;
use crate::{
    ERROR_INTERRUPTED_WRITTEN, ERROR_MAY_HAVE_CHANGED, IntentItem, IntentSetting, PlanItem,
    TweakChange, TweakId,
};

/// Layout of a `JournalEntry` file.
pub const TWEAK_JOURNAL_VERSION: u8 = 1;

/// What the elevated helper recorded about one item before it wrote anything: the settings it was
/// to change, each with the value read and the value to write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub ts_ms: i64,
    pub tweak: TweakId,
    pub subject: Option<String>,
    /// The entry this one puts back, when the item is an undo.
    pub undoes: Option<u64>,
    pub settings: Vec<IntentSetting>,
}

impl JournalEntry {
    /// The entry of one item of an intent; an undo names the entry it puts back.
    pub fn of(item: &IntentItem, ts_ms: i64, undoes: Option<u64>) -> JournalEntry {
        JournalEntry {
            ts_ms,
            tweak: item.tweak,
            subject: item.subject.clone(),
            undoes,
            settings: item.settings.clone(),
        }
    }
}

/// A folder only administrators can write, where the helper records what it changes.
pub trait TweakJournal {
    /// The entry under `id`: `None` when the journal holds none, an error when it cannot be read.
    fn entry(&self, id: u64) -> Result<Option<JournalEntry>, String>;
    /// Stores `entry` under a new id and puts it on disk before returning that id.
    fn record(&mut self, entry: &JournalEntry) -> Result<u64, String>;
}

/// A journal that holds nothing and records nothing.
pub struct NoJournal;

impl TweakJournal for NoJournal {
    fn entry(&self, _id: u64) -> Result<Option<JournalEntry>, String> {
        Ok(None)
    }

    fn record(&mut self, _entry: &JournalEntry) -> Result<u64, String> {
        Err("there is no journal".into())
    }
}

/// Whether an undo item may run: one that names a journal entry must match it (same tweak, only its
/// settings, values put back as read before); one that names none predates the journal.
pub fn check_undo(item: &PlanItem, journal: &impl TweakJournal) -> Result<(), String> {
    let (Some(_), Some(id)) = (item.undo_of, item.undo_journal) else {
        return Ok(());
    };
    let entry = journal
        .entry(id)?
        .ok_or_else(|| format!("the journal holds no change {id}"))?;
    if entry.undoes.is_some() {
        return Err(format!("change {id} is itself an undo"));
    }
    if entry.tweak != item.tweak || entry.subject != item.subject {
        return Err(format!("change {id} is not of this tweak"));
    }
    for change in &item.changes {
        let held = entry
            .settings
            .iter()
            .find(|held| held.setting == change.setting)
            .ok_or_else(|| format!("change {id} did not touch {:?}", change.setting))?;
        let restores = held
            .before
            .as_ref()
            .is_some_and(|before| same_state(before, &change.to));
        if !restores || !same_state(&held.to, &change.from) {
            return Err(format!(
                "change {id} did not write the values this undo names"
            ));
        }
    }
    Ok(())
}

/// Whether the history may keep a change: its journal entry holds the settings and values it wrote,
/// or it wrote nothing, like a refusal.
pub fn backed_by_journal(
    change: &TweakChange,
    journal: &impl TweakJournal,
) -> Result<bool, String> {
    let Some(id) = change.journal else {
        return Ok(writes_nothing(change));
    };
    let Some(entry) = journal.entry(id)? else {
        return Ok(false);
    };
    Ok(entry.tweak == change.tweak
        && entry.subject == change.subject
        && entry.undoes.is_some() == change.undo_of.is_some()
        && change.results.iter().all(|result| {
            entry.settings.iter().any(|held| {
                held.setting == result.change.setting && same_state(&held.to, &result.change.to)
            })
        }))
}

/// A change whose every result failed without possibly having written: it moves nothing.
fn writes_nothing(change: &TweakChange) -> bool {
    !change.results.is_empty()
        && change.results.iter().all(|result| {
            result.error.is_some_and(|code| {
                code != ERROR_MAY_HAVE_CHANGED && code != ERROR_INTERRUPTED_WRITTEN
            })
        })
}
