//! Event Tracing for Windows: an own system session and its real-time consumer.

mod consumer;
mod kernel;
mod session;

pub use consumer::{Consumed, EventSink, spawn_consumer};
pub use kernel::{IMAGE, KernelRecord, PERF_INFO, kernel_record};
pub use session::{DEEP_CAPTURE_GUID, DEEP_CAPTURE_SESSION, KernelSession, SessionStats};

use windows::core::GUID;

/// One event as the consumer hands it over; `timestamp` is raw QPC.
#[derive(Clone, Copy, Debug)]
pub struct RawEvent<'a> {
    pub provider: GUID,
    pub opcode: u8,
    pub version: u8,
    pub cpu: u16,
    pub timestamp: i64,
    pub data: &'a [u8],
}
