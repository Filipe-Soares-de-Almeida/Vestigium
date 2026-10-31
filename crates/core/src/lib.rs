//! Domain types and rules for Vestigium.

mod gate;
mod process;
mod sample;

pub use gate::{GateState, RunGate};
pub use process::{ProcessCounters, ProcessSample, ProcessTop, top_processes};
pub use sample::{CoreLoad, DiskLoad, Sample};
