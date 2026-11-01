use serde::{Deserialize, Serialize};

use crate::{Cause, Fact, Severity, Suspect, Trigger, Window};

/// What kind of stall an occurrence was, named after its gravest trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OccurrenceKind {
    Freeze,
    Stutter,
    AppHang,
    Reported,
}

impl OccurrenceKind {
    pub fn of(trigger: &Trigger) -> OccurrenceKind {
        match trigger {
            Trigger::Freeze { .. } => OccurrenceKind::Freeze,
            Trigger::Stutter { .. } => OccurrenceKind::Stutter,
            Trigger::AppHang { .. } => OccurrenceKind::AppHang,
            Trigger::Reported { .. } => OccurrenceKind::Reported,
        }
    }

    /// Freeze is gravest, then a hung app, a stutter, and an event alone.
    pub fn rank(self) -> u8 {
        match self {
            OccurrenceKind::Freeze => 3,
            OccurrenceKind::AppHang => 2,
            OccurrenceKind::Stutter => 1,
            OccurrenceKind::Reported => 0,
        }
    }
}

/// One stall as stored: what set it off, what was watched around it, and who is suspected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Occurrence {
    pub kind: OccurrenceKind,
    pub start_ms: i64,
    pub end_ms: i64,
    pub severity: Severity,
    pub triggers: Vec<Trigger>,
    pub window: Window,
    pub suspects: Vec<Suspect>,
    pub facts: Vec<Fact>,
    /// Sensitive mode was on while it happened.
    pub sensitive: bool,
    /// Watching stopped before the window after it was complete.
    pub truncated: bool,
}

impl Occurrence {
    /// The cause of the most likely suspect.
    pub fn cause(&self) -> Cause {
        self.suspects
            .first()
            .map_or(Cause::Unclear, |s| s.cause.clone())
    }
}

/// What a list of occurrences shows: no window, only the top suspect's cause.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OccurrenceSummary {
    pub id: i64,
    pub kind: OccurrenceKind,
    pub start_ms: i64,
    pub end_ms: i64,
    pub severity: Severity,
    pub cause: Cause,
}
