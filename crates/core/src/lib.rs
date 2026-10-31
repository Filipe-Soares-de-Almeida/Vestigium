//! Domain types and rules for Argos.

mod gate;
mod process;
mod sample;

pub use gate::{GateState, RunGate};
pub use process::{ProcessCounters, ProcessSample, ProcessTop, top_processes};
pub use sample::{CoreLoad, DiskLoad, Sample};
