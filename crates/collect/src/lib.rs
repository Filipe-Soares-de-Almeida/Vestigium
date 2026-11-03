//! Windows data sources of Vestigium.

mod blackbox;
mod boot;
mod cpu;
mod disk;
mod dumps;
mod etw;
mod events;
mod gpu;
mod health;
mod hung;
mod hwtest;
mod memory;
mod pdh;
mod presentmon;
mod probe;
mod processes;
mod timer;
mod tweaks;

pub use blackbox::{BlackboxRing, RING_RECORDS, RingContents};
pub use boot::{BOOT_LOG, boot_report};
pub use cpu::{CoreTimes, CpuReading, CpuSampler, core_loads};
pub use disk::{disk_media, physical_drive};
pub use dumps::{CrashControl, DumpFile, app_dump_dir, crash_control, dump_file, list_dumps};
pub use etw::{
    DEEP_CAPTURE_GUID, DEEP_CAPTURE_SESSION, EventSink, IMAGE, KernelRecord, KernelSession,
    PERF_INFO, RawEvent, SessionStats, kernel_record, spawn_consumer,
};
pub use events::{Classifier, Delivered, EventSubscription};
pub use gpu::{GpuReading, gpu_usage};
pub use health::{
    HealthSource, WindowsHealth, antivirus, disk_bus, disk_health, disk_nvme, page_files,
    system_drive, volume_drive, volumes,
};
pub use hung::hung_pids;
pub use hwtest::{
    BenchTarget, LockedRegion, MemoryProgress, PatternRun, Words, benchmark,
    memory_diagnostic_events, memory_test, run_patterns,
};
pub use memory::{MemoryInfo, logical_cpus, memory_info};
pub use pdh::{PdhReading, PdhSampler};
pub use presentmon::{
    CsvColumns, FrameRow, PRESENTMON_SESSION, PresentMonOutcome, presentmon_args, run_presentmon,
};
pub use probe::{ProbeConfig, spawn_probe};
pub use processes::ProcessSnapshotter;
pub use timer::{HighResTimer, monotonic_ms, unbiased_ms, wall_ms};
pub use tweaks::{
    RESTORE_CREATED, RESTORE_SKIPPED, RestoreEvent, WindowsSettings, exe_of, has_battery,
    hiberfile_bytes, monitor_count, read_setting, restore_event, restore_events, startup_entries,
    write_setting,
};

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
