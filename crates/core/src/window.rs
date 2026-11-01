use serde::{Deserialize, Serialize};

use crate::{HangOutcome, MachineInfo, ProcessTop, Sample, WinEvent};

/// Why watching has a hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapReason {
    Paused,
    Suspended,
    Stopped,
}

/// A span with nothing watched; nothing inside it counts as a stall.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gap {
    pub start_ms: i64,
    pub end_ms: i64,
    pub reason: GapReason,
}

/// A process that stopped responding, from the first sign to the check that saw it end.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HangSpan {
    pub pid: u32,
    pub name: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub outcome: HangOutcome,
}

/// Everything watched around an occurrence, oldest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub from_ms: i64,
    pub to_ms: i64,
    pub samples: Vec<Sample>,
    pub tops: Vec<ProcessTop>,
    pub events: Vec<WinEvent>,
    pub hangs: Vec<HangSpan>,
    pub gaps: Vec<Gap>,
    pub machine: MachineInfo,
}
