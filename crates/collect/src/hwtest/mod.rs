mod bench;
mod memdiag;
mod memory;

pub use bench::{BenchTarget, benchmark, benchmark_with_timeout, disk_io_bytes};
pub use memdiag::memory_diagnostic_events;
pub use memory::{LockedRegion, MemoryProgress, PatternRun, Words, memory_test, run_patterns};
