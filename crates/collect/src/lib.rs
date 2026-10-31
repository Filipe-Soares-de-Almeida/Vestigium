//! Windows data sources of WatchMyPC.

mod probe;
mod timer;

pub use probe::spawn_probe;
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
