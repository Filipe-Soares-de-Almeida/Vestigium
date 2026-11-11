use std::time::Duration;

use serde::{Deserialize, Serialize};
use vestigium_core::health::FindingKind;
use vestigium_core::{
    EpisodeKind, HungWindow, OccurrenceSummary, PlanItem, PressureSpan, ProcessTop, ProgramPoint,
    Reading, Sample, Settings, TweakChange,
};

/// Version of the messages below; both ends refuse a different one.
pub const PROTOCOL_VERSION: u32 = 21;

/// How long the watcher may take to answer a request that only reads memory or flips a flag.
pub const ANSWER_SHORT: Duration = Duration::from_secs(3);
/// How long it may take to answer a request that works on the history or on Windows.
pub const ANSWER_LONG: Duration = Duration::from_secs(60);

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
    /// One program's use at each ranking of the last ten minutes.
    ProgramRecent {
        name: String,
    },
    SetPaused {
        paused: bool,
        /// When a pause ends by itself, Unix epoch milliseconds; none holds it until resumed or
        /// until the watcher restarts.
        #[serde(default)]
        until_ms: Option<i64>,
    },
    SetSensitive {
        on: bool,
    },
    Settings,
    SetSettings(Settings),
    SetAutostart {
        on: bool,
    },
    /// Deletes everything watched so far; settings stay.
    ClearHistory,
    /// Stores the deep captures, health scans, dump reads, dump deletions and tweak results the
    /// helper left in the data folder.
    ImportResults,
    /// Applies or undoes a tweak of the signed-in user's own settings and keeps it in the history.
    ApplyUserTweak {
        item: PlanItem,
    },
}

impl Request {
    /// How long the answer to this request may take before the sender calls the watcher mute.
    pub fn answer_deadline(&self) -> Duration {
        match self {
            Request::SetSettings(_)
            | Request::ClearHistory
            | Request::ImportResults
            | Request::ApplyUserTweak { .. } => ANSWER_LONG,
            Request::Hello { .. }
            | Request::Status
            | Request::Now
            | Request::Recent
            | Request::ProgramRecent { .. }
            | Request::SetPaused { .. }
            | Request::SetSensitive { .. }
            | Request::Settings
            | Request::SetAutostart { .. } => ANSWER_SHORT,
        }
    }
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
    /// Oldest first; empty when the program ran in none of those rankings.
    ProgramRecent {
        points: Vec<ProgramPoint>,
    },
    Settings {
        settings: Settings,
        autostart: bool,
    },
    /// The ids of the deep captures, health scans, dumps and tweak changes stored by an import, in
    /// the order read, how many dumps it marked as deleted, and how many deletions the helper
    /// refused or could not carry out.
    Imported {
        captures: Vec<i64>,
        scans: Vec<i64>,
        dumps: Vec<i64>,
        deleted: u32,
        refused: u32,
        failed: u32,
        tweaks: Vec<i64>,
        tests: Vec<i64>,
    },
    /// A user tweak was carried out; `id` is its history row.
    TweakApplied {
        id: i64,
        change: TweakChange,
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
    /// When the pause ends by itself, while paused; Unix epoch milliseconds.
    #[serde(default)]
    pub paused_until_ms: Option<i64>,
    /// When sensitive mode turns itself off, while it is on; Unix epoch milliseconds.
    pub sensitive_until_ms: Option<i64>,
    pub state: TrayState,
    /// Unix epoch, milliseconds.
    pub started_at_ms: i64,
    pub last_sample_ms: Option<i64>,
    pub last_error: Option<String>,
    /// Sensitive mode saw hitches worth a deep capture.
    #[serde(default)]
    pub suggest_capture: bool,
    /// The agent's own share of the whole machine over the last minute, 0 to 100.
    #[serde(default)]
    pub cpu_pct: Option<f32>,
    /// The machine went to sleep and has not woken up again.
    #[serde(default)]
    pub suspended: bool,
    /// How the watching itself is going.
    #[serde(default)]
    pub monitor: MonitorStatus,
}

/// Whether the agent is reading the machine, as the tray and the window say it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorState {
    /// Reading, or paused or suspended on purpose.
    #[default]
    Watching,
    /// Reading, but a source fails.
    Degraded,
    /// No sample for a while although nothing stopped the watching.
    NoReading,
}

/// A data source the sampler reads every second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorSource {
    Memory,
    Cpu,
    /// The performance counters: disks, GPU and the rest.
    Counters,
    Processes,
    Timer,
}

/// A source that fails now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceFailure {
    pub source: MonitorSource,
    /// Unix epoch, milliseconds.
    pub since_ms: i64,
    pub error: String,
}

/// The agent's account of its own watching.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MonitorStatus {
    pub state: MonitorState,
    /// Sources failing now, each with its error; empty when all read.
    #[serde(default)]
    pub failing: Vec<SourceFailure>,
    /// Unix epoch, milliseconds: the last sample, or the last start or resume when that is newer.
    /// Set only while the state is no reading.
    #[serde(default)]
    pub no_reading_since_ms: Option<i64>,
    /// When watching last started or resumed; a sample is expected a few seconds after it.
    #[serde(default)]
    pub resumed_ms: Option<i64>,
    /// When the latest health pass was asked for, and when one last finished; Unix epoch ms.
    #[serde(default)]
    pub health_attempt_ms: Option<i64>,
    #[serde(default)]
    pub health_ok_ms: Option<i64>,
    /// Finding kinds the latest finished pass could not read.
    #[serde(default)]
    pub health_unread: Vec<FindingKind>,
}

/// The latest second watched.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NowFrame {
    pub sample: Option<Sample>,
    /// The busiest programs, each summing its processes under the pid of the busiest one.
    pub top: Option<ProcessTop>,
    /// Every program running, summed the same way, busiest on the processor first.
    pub programs: Option<ProcessTop>,
    /// Processes with a window not responding at the last check.
    pub hung: Vec<HungWindow>,
    /// The most recent occurrence stored.
    pub latest: Option<OccurrenceSummary>,
    /// CPU, disk and memory pressure episodes under way; hints are left out.
    pub pressure: Vec<EpisodeKind>,
}
