use serde::{Deserialize, Serialize};

use crate::{Cause, Evidence, Fact, Severity, Suspect, Trigger, Window};

/// What kind of stall an occurrence was, named after its gravest trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OccurrenceKind {
    Freeze,
    Stutter,
    AppHang,
    Reported,
    /// The PC went down while watched and came back with a reboot.
    PostMortem,
}

impl OccurrenceKind {
    /// Every variant, in declaration order.
    pub const ALL: [OccurrenceKind; 5] = [
        OccurrenceKind::Freeze,
        OccurrenceKind::Stutter,
        OccurrenceKind::AppHang,
        OccurrenceKind::Reported,
        OccurrenceKind::PostMortem,
    ];
}

impl OccurrenceKind {
    pub fn of(trigger: &Trigger) -> OccurrenceKind {
        match trigger {
            Trigger::Freeze { .. } => OccurrenceKind::Freeze,
            Trigger::Stutter { .. } => OccurrenceKind::Stutter,
            Trigger::AppHang { .. } => OccurrenceKind::AppHang,
            Trigger::Reported { .. } => OccurrenceKind::Reported,
            Trigger::Crash { .. } => OccurrenceKind::PostMortem,
        }
    }

    /// A fall is gravest, then a freeze, a hung app, a stutter, and an event alone.
    pub fn rank(self) -> u8 {
        match self {
            OccurrenceKind::PostMortem => 4,
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
    /// Watching stopped before the window after it was complete, or the first seconds of the
    /// window had already left the rings.
    pub truncated: bool,
}

impl Occurrence {
    /// The cause of the most likely suspect.
    pub fn cause(&self) -> Cause {
        self.suspects
            .first()
            .map_or(Cause::Unclear, |s| s.cause.clone())
    }

    /// Whether `program` had a window that hung here, or a suspect names it as cause or evidence.
    pub fn involves(&self, program: &str) -> bool {
        let named = |name: &str| same_program(name, program);
        self.window.hangs.iter().any(|h| named(&h.name))
            || self.suspects.iter().any(|s| {
                cause_programs(&s.cause).into_iter().any(named)
                    || s.evidence.iter().any(|e| match e {
                        Evidence::Process { name, .. } | Evidence::Hang { name, .. } => named(name),
                        _ => false,
                    })
            })
    }
}

/// The programs a cause names.
fn cause_programs(cause: &Cause) -> Vec<&str> {
    match cause {
        Cause::DiskIo {
            process: Some(name),
            ..
        }
        | Cause::Memory {
            process: Some(name),
        }
        | Cause::App { name }
        | Cause::Frames { app: name } => vec![name.as_str()],
        Cause::Cpu { processes } => processes.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Whether two names are the same program, ignoring case and the ".exe" ending.
pub fn same_program(a: &str, b: &str) -> bool {
    let bare = |name: &str| {
        let lower = name.to_lowercase();
        lower
            .strip_suffix(".exe")
            .map(str::to_string)
            .unwrap_or(lower)
    };
    bare(a) == bare(b)
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
