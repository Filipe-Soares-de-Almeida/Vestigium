//! Domain types and rules for Vestigium.

mod event;
mod gate;
mod machine;
mod minute;
mod probe;
mod process;
mod sample;

pub use event::{
    CURATED, Channel, CuratedEvent, EventMeaning, WinEvent, classify, query_xml, query_xml_for,
};
pub use gate::{GateState, RunGate};
pub use machine::{DiskInfo, MachineInfo, Media};
pub use minute::{DiskMinute, MINUTE_MS, MinuteRollup, MinuteSample, minute_of, summarize_minute};
pub use probe::{DEFAULT_PROBE_PERIOD, ProbeTick, SENSITIVE_FOR, SENSITIVE_PROBE_PERIOD};
pub use process::{
    ProcessCounters, ProcessSample, ProcessSampleV1, ProcessTop, ProcessTopV1, top_processes,
};
pub use sample::{CoreLoad, DiskLoad, Sample, SampleV1};
