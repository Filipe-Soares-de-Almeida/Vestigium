//! Windows data sources of Argos.

mod memory;
mod pdh;
mod probe;
mod processes;
mod timer;

pub use memory::{MemoryInfo, logical_cpus, memory_info};
pub use pdh::{CpuInstance, PdhReading, PdhSampler, parse_cpu_instance};
pub use probe::spawn_probe;
pub use processes::ProcessSnapshotter;
pub use timer::{HighResTimer, monotonic_ms};

/// Why a data source failed.
#[derive(Debug, thiserror::Error)]
pub enum CollectError {
    #[error("Windows error: {0}")]
    Win32(#[from] windows::core::Error),
    #[error("{call} failed with PDH status {status:#010x}")]
    Pdh { call: &'static str, status: u32 },
    #[error("cannot start a thread: {0}")]
    Io(#[from] std::io::Error),
}
