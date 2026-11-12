use serde::{Deserialize, Serialize};

/// One second of system state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Unix epoch, milliseconds.
    pub ts_ms: i64,
    pub cpu_pct: f32,
    pub dpc_pct: f32,
    pub isr_pct: f32,
    /// Performance ceiling as a share of nominal; 100 means not held back. `None` when the
    /// counter could not be read this second.
    pub perf_limit_pct: Option<f32>,
    pub proc_perf_pct: Option<f32>,
    pub mem_total_bytes: u64,
    pub mem_avail_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
    pub hard_faults_per_s: Option<f32>,
    /// Busiest GPU engine; `None` on a machine without GPU Engine counters.
    pub gpu_pct: Option<f32>,
    /// Worst stall-probe delay seen during this second.
    pub probe_max_lag_ms: f32,
    /// Delay of this sample's own tick, a sign of CPU contention.
    pub sampler_lag_ms: f32,
    pub cores: Vec<CoreLoad>,
    pub disks: Vec<DiskLoad>,
}

/// Load of one logical processor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreLoad {
    pub cpu_pct: f32,
    pub dpc_pct: f32,
    pub isr_pct: f32,
}

/// Load of one physical disk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiskLoad {
    /// Instance name as Windows reports it, such as "0 C:".
    pub name: String,
    pub active_pct: f32,
    /// `None` for a counter that could not be read this second.
    pub latency_ms: Option<f32>,
    pub queue_len: Option<f32>,
    pub bytes_per_s: Option<f32>,
}

/// `DiskLoad` before its counters could be absent, kept to read older history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiskLoadV2 {
    pub name: String,
    pub active_pct: f32,
    pub latency_ms: f32,
    pub queue_len: f32,
    pub bytes_per_s: f32,
}

impl From<DiskLoadV2> for DiskLoad {
    fn from(old: DiskLoadV2) -> DiskLoad {
        DiskLoad {
            name: old.name,
            active_pct: old.active_pct,
            latency_ms: Some(old.latency_ms),
            queue_len: Some(old.queue_len),
            bytes_per_s: Some(old.bytes_per_s),
        }
    }
}

/// The layout of `Sample` from before its performance counters could be absent, kept to read
/// older history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleV2 {
    pub ts_ms: i64,
    pub cpu_pct: f32,
    pub dpc_pct: f32,
    pub isr_pct: f32,
    pub perf_limit_pct: f32,
    pub proc_perf_pct: f32,
    pub mem_total_bytes: u64,
    pub mem_avail_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
    pub hard_faults_per_s: f32,
    pub gpu_pct: Option<f32>,
    pub probe_max_lag_ms: f32,
    pub sampler_lag_ms: f32,
    pub cores: Vec<CoreLoad>,
    pub disks: Vec<DiskLoadV2>,
}

impl From<SampleV2> for Sample {
    fn from(old: SampleV2) -> Sample {
        Sample {
            ts_ms: old.ts_ms,
            cpu_pct: old.cpu_pct,
            dpc_pct: old.dpc_pct,
            isr_pct: old.isr_pct,
            perf_limit_pct: Some(old.perf_limit_pct),
            proc_perf_pct: Some(old.proc_perf_pct),
            mem_total_bytes: old.mem_total_bytes,
            mem_avail_bytes: old.mem_avail_bytes,
            commit_bytes: old.commit_bytes,
            commit_limit_bytes: old.commit_limit_bytes,
            hard_faults_per_s: Some(old.hard_faults_per_s),
            gpu_pct: old.gpu_pct,
            probe_max_lag_ms: old.probe_max_lag_ms,
            sampler_lag_ms: old.sampler_lag_ms,
            cores: old.cores,
            disks: old.disks.into_iter().map(Into::into).collect(),
        }
    }
}

/// The first stored layout of `Sample`, kept to read older history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleV1 {
    pub ts_ms: i64,
    pub cpu_pct: f32,
    pub dpc_pct: f32,
    pub isr_pct: f32,
    pub perf_limit_pct: f32,
    pub proc_perf_pct: f32,
    pub mem_avail_bytes: u64,
    pub commit_bytes: u64,
    pub commit_limit_bytes: u64,
    pub hard_faults_per_s: f32,
    pub probe_max_lag_ms: f32,
    pub sampler_lag_ms: f32,
    pub cores: Vec<CoreLoad>,
    pub disks: Vec<DiskLoadV2>,
}

impl From<SampleV1> for Sample {
    fn from(old: SampleV1) -> Sample {
        Sample {
            ts_ms: old.ts_ms,
            cpu_pct: old.cpu_pct,
            dpc_pct: old.dpc_pct,
            isr_pct: old.isr_pct,
            perf_limit_pct: Some(old.perf_limit_pct),
            proc_perf_pct: Some(old.proc_perf_pct),
            mem_total_bytes: 0,
            mem_avail_bytes: old.mem_avail_bytes,
            commit_bytes: old.commit_bytes,
            commit_limit_bytes: old.commit_limit_bytes,
            hard_faults_per_s: Some(old.hard_faults_per_s),
            gpu_pct: None,
            probe_max_lag_ms: old.probe_max_lag_ms,
            sampler_lag_ms: old.sampler_lag_ms,
            cores: old.cores,
            disks: old.disks.into_iter().map(Into::into).collect(),
        }
    }
}
