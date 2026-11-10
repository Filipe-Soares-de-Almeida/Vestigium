use serde::{Deserialize, Serialize};

use crate::{CrashKind, WinEvent};

/// How bad a stall was, by how long it lasted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Light,
    Medium,
    Severe,
}

impl Severity {
    /// Every variant, in declaration order.
    pub const ALL: [Severity; 3] = [Severity::Light, Severity::Medium, Severity::Severe];
}

/// Under 1 s is light, up to 5 s medium, longer severe.
pub fn severity(duration_ms: f32) -> Severity {
    if duration_ms < 1_000.0 {
        Severity::Light
    } else if duration_ms <= 5_000.0 {
        Severity::Medium
    } else {
        Severity::Severe
    }
}

/// How an app that stopped responding came out of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HangOutcome {
    Recovered,
    Closed,
}

/// Something that starts an occurrence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Trigger {
    Freeze {
        start_ms: i64,
        end_ms: i64,
        lag_ms: f32,
    },
    Stutter {
        start_ms: i64,
        end_ms: i64,
        count: u16,
        worst_ms: f32,
    },
    AppHang {
        pid: u32,
        name: String,
        start_ms: i64,
        end_ms: i64,
        outcome: HangOutcome,
    },
    Reported {
        event: WinEvent,
    },
    /// The PC went down while watched: `last_ms` is the last stored sample, `down_ms` when
    /// Windows says it fell.
    Crash {
        last_ms: i64,
        down_ms: i64,
        kind: CrashKind,
    },
}

impl Trigger {
    pub fn start_ms(&self) -> i64 {
        match self {
            Trigger::Freeze { start_ms, .. }
            | Trigger::Stutter { start_ms, .. }
            | Trigger::AppHang { start_ms, .. } => *start_ms,
            Trigger::Reported { event } => event.ts_ms,
            Trigger::Crash { last_ms, .. } => *last_ms,
        }
    }

    pub fn end_ms(&self) -> i64 {
        match self {
            Trigger::Freeze { end_ms, .. }
            | Trigger::Stutter { end_ms, .. }
            | Trigger::AppHang { end_ms, .. } => *end_ms,
            Trigger::Reported { event } => event.ts_ms,
            Trigger::Crash {
                last_ms, down_ms, ..
            } => (*down_ms).max(*last_ms),
        }
    }
}
