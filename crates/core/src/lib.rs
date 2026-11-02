//! Domain types and rules for Vestigium.

mod blackbox;
mod episode;
mod event;
mod facts;
mod freeze;
mod gate;
mod hang;
mod machine;
mod minute;
mod occurrence;
mod probe;
mod process;
mod reading;
mod ring;
mod rules;
mod sample;
mod settings;
mod stutter;
mod trigger;
mod window;

pub use blackbox::{
    Blackbox, ECHOING_FREEZE_MS, GAP_MARGIN_MS, HANG_ECHO_MS, Notable, Output, RING_MS,
    RecentState, Timing,
};
pub use episode::{Culprit, Episode, EpisodeDetector, EpisodeKind, HINT_EVERY_MS, QUIET_MS};
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
pub use stutter::{BURST_COUNT, BURST_WINDOW_MS, HICCUP_MS, StutterDetector};
pub use trigger::{HangOutcome, Severity, Trigger, severity};
pub use window::{Gap, GapReason, HangSpan, Window};
