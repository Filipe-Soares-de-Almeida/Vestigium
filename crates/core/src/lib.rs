//! Domain types and rules for Vestigium.

mod blackbox;
mod capture;
mod dump;
mod episode;
mod event;
mod facts;
mod freeze;
mod gate;
mod hang;
pub mod health;
mod machine;
mod minute;
mod occurrence;
mod postmortem;
mod probe;
mod process;
mod reading;
mod ring;
mod rules;
mod sample;
mod settings;
mod shutdown;
mod stutter;
mod trigger;
mod window;
mod xml;

pub use blackbox::{
    Blackbox, ECHOING_FREEZE_MS, GAP_MARGIN_MS, HANG_ECHO_MS, Notable, Output, RING_MS,
    RecentState, Timing,
};
pub use capture::{
    AppFrames, BUCKET_BOUNDS_US, BUCKETS, CAPTURE_VERSION, CaptureSource, DeepCapture,
    DriverLatency, FrameAggregator, FrameHitch, HITCH_FACTOR, HITCH_MIN_MS, HITCHES_KEPT,
    LONGEST_KEPT, LongRoutine, ModuleMap, ON_DEMAND_SHARE, ProbeHiccup, RoutineAggregator,
    RoutineKind, RoutineStats, capture_findings, capture_suspects, is_compositor, is_intermediary,
    is_kernel_core, renders_steadily, steady_only,
};
pub use dump::{
    Analysis, DUMP_FALL_REACH_MS, DUMP_VERSION, DumpCulprit, DumpKind, DumpRecord, Fault,
    bug_check_report, dump_culprit, dump_of_fall, fault_address, kernel_dump_kind,
};
pub use episode::{
    Culprit, Episode, EpisodeDetector, EpisodeKind, HINT_EVERY_MS, PressureSpan, QUIET_MS,
};
pub use event::{
    CURATED, Channel, CuratedEvent, EventMeaning, WinEvent, classify, query_xml, query_xml_for,
};
pub use facts::{Fact, FactKind, facts};
pub use freeze::{FREEZE_MS, freeze};
pub use gate::{GateState, RunGate};
pub use hang::{HUNG_AFTER_MS, HangTracker, HungWindow};
pub use machine::{DiskInfo, MachineInfo, Media};
pub use minute::{DiskMinute, MINUTE_MS, MinuteRollup, MinuteSample, minute_of, summarize_minute};
pub use occurrence::{Occurrence, OccurrenceKind, OccurrenceSummary};
pub use postmortem::{POSTMORTEM_BEFORE_MS, POSTMORTEM_REACH_MS, postmortem};
pub use probe::{DEFAULT_PROBE_PERIOD, ProbeTick, SENSITIVE_FOR, SENSITIVE_PROBE_PERIOD};
pub use process::{
    ProcessCounters, ProcessSample, ProcessSampleV1, ProcessTop, ProcessTopV1, heaviest, last_seen,
    process_rates, programs,
};
pub use reading::{BUSY_ABOVE, PcState, Reading, pc_state};
pub use ring::Ring;
pub use rules::{
    Cause, Confidence, Evidence, Metric, NextStep, Suspect, crosses_down, suspects, threshold,
};
pub use sample::{CoreLoad, DiskLoad, Sample, SampleV1};
pub use settings::{
    ColorTheme, DetailLevel, LanguagePref, Retention, SettingError, Settings, ThemePref,
};
pub use shutdown::{
    CrashKind, PowerLossDetail, SHUTTING_DOWN, power_loss_detail, unexpected_shutdown_ms,
};
pub use stutter::{BURST_COUNT, BURST_WINDOW_MS, HICCUP_MS, StutterDetector};
pub use trigger::{HangOutcome, Severity, Trigger, severity};
pub use window::{Gap, GapReason, HangSpan, Window};
