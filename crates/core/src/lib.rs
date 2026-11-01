//! Domain types and rules for Vestigium.

mod episode;
mod event;
mod freeze;
mod gate;
mod hang;
mod machine;
mod minute;
mod probe;
mod process;
mod sample;
mod stutter;
mod trigger;

pub use episode::{Culprit, Episode, EpisodeDetector, EpisodeKind, HINT_EVERY_MS, QUIET_MS};
pub use event::{
    CURATED, Channel, CuratedEvent, EventMeaning, WinEvent, classify, query_xml, query_xml_for,
};
pub use freeze::{FREEZE_MS, freeze};
pub use gate::{GateState, RunGate};
pub use hang::{HUNG_AFTER_MS, HangTracker, HungWindow};
pub use machine::{DiskInfo, MachineInfo, Media};
pub use minute::{DiskMinute, MINUTE_MS, MinuteRollup, MinuteSample, minute_of, summarize_minute};
pub use probe::{DEFAULT_PROBE_PERIOD, ProbeTick, SENSITIVE_FOR, SENSITIVE_PROBE_PERIOD};
pub use process::{
    ProcessCounters, ProcessSample, ProcessSampleV1, ProcessTop, ProcessTopV1, top_processes,
};
pub use sample::{CoreLoad, DiskLoad, Sample, SampleV1};
pub use stutter::{BURST_COUNT, BURST_WINDOW_MS, HICCUP_MS, StutterDetector};
pub use trigger::{HangOutcome, Severity, Trigger, severity};
