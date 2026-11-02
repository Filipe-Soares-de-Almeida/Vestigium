use serde::{Deserialize, Serialize};
use vestigium_core::{
    EpisodeKind, HungWindow, OccurrenceSummary, PressureSpan, ProcessTop, Reading, Sample, Settings,
};

/// Version of the messages below; both ends refuse a different one.
pub const PROTOCOL_VERSION: u32 = 4;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Hello {
        protocol: u32,
    },
    Status,
    /// The latest second watched, for the live screen.
    Now,
    /// The last ten minutes, so the live screen opens with them drawn.
    Recent,
    SetPaused {
        paused: bool,
    },
    SetSensitive {
        on: bool,
    },
    Settings,
    SetSettings(Settings),
    SetAutostart {
        on: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Hello {
        protocol: u32,
        agent_version: String,
    },
    Status(AgentStatus),
    Now(Box<NowFrame>),
    /// One reading a second, oldest first.
    Recent {
        readings: Vec<Reading>,
        /// CPU, disk and memory pressure episodes of those minutes, ended or under way; hints are
        /// left out.
        pressure: Vec<PressureSpan>,
    },
    Settings {
        settings: Settings,
        autostart: bool,
    },
    /// The command was carried out.
    Done,
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// A request this agent does not speak.
    Protocol,
    InvalidSetting,
    Failed,
}

/// What the tray shows about the last 24 h.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayState {
    #[default]
    Normal,
    /// Some occurrence or pressure episode.
    Attention,
    /// A severe occurrence.
    Problem,
}

/// What the agent is doing right now.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub paused: bool,
    /// When sensitive mode turns itself off, while it is on; Unix epoch milliseconds.
    pub sensitive_until_ms: Option<i64>,
    pub state: TrayState,
    /// Unix epoch, milliseconds.
    pub started_at_ms: i64,
    pub last_sample_ms: Option<i64>,
    pub last_error: Option<String>,
}

/// The latest second watched.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NowFrame {
    pub sample: Option<Sample>,
    /// The busiest programs, each summing its processes under the pid of the busiest one.
    pub top: Option<ProcessTop>,
    /// Processes with a window not responding at the last check.
    pub hung: Vec<HungWindow>,
    /// The most recent occurrence stored.
    pub latest: Option<OccurrenceSummary>,
    /// CPU, disk and memory pressure episodes under way; hints are left out.
    pub pressure: Vec<EpisodeKind>,
}
