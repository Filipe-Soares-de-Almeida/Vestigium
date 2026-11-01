use serde::{Deserialize, Serialize};

use crate::WinEvent;

/// How bad a stall was, by how long it lasted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Light,
    Medium,
    Severe,
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
}
