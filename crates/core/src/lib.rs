//! Domain types and rules for Vestigium.

mod gate;
mod minute;
mod probe;
mod process;
mod sample;

pub use gate::{GateState, RunGate};
pub use minute::{DiskMinute, MINUTE_MS, MinuteRollup, MinuteSample, minute_of, summarize_minute};
pub use probe::{DEFAULT_PROBE_PERIOD, ProbeTick, SENSITIVE_FOR, SENSITIVE_PROBE_PERIOD};
pub use process::{
    ProcessCounters, ProcessSample, ProcessSampleV1, ProcessTop, ProcessTopV1, top_processes,
};
pub use sample::{CoreLoad, DiskLoad, Sample, SampleV1};
