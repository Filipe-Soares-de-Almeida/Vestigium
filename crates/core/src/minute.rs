use serde::{Deserialize, Serialize};

use crate::Sample;

/// Length of one rollup, in milliseconds.
pub const MINUTE_MS: i64 = 60_000;

/// One wall-clock minute of samples, for the day view and the long history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MinuteSample {
    /// Start of the minute, Unix epoch milliseconds.
    pub ts_ms: i64,
    /// Below 60 when watching started, paused or stopped inside the minute.
    pub samples: u16,
    pub cpu_avg: f32,
    pub cpu_max: f32,
    /// Busiest logical processor.
    pub core_max: f32,
    /// Highest DPC plus interrupt time on one logical processor.
    pub dpc_isr_max: f32,
    pub mem_total_bytes: u64,
    pub mem_avail_min: u64,
    pub commit_max: u64,
    pub hard_faults_max: f32,
    pub gpu_max: Option<f32>,
    /// Lowest performance ceiling; 100 means the processor was never held back.
    pub perf_limit_min: f32,
    pub probe_max_lag_ms: f32,
    pub disks: Vec<DiskMinute>,
}

/// One physical disk over one minute.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiskMinute {
    pub name: String,
    pub active_max: f32,
    pub latency_max_ms: f32,
    pub bytes_avg: f32,
}

/// Start of the wall-clock minute holding `ts_ms`; saturates at the ends of the range.
pub fn minute_of(ts_ms: i64) -> i64 {
    ts_ms.saturating_sub(ts_ms.rem_euclid(MINUTE_MS))
}

/// Summary of samples that all fall in one minute; `None` for no samples.
pub fn summarize_minute(samples: &[Sample]) -> Option<MinuteSample> {
    let mut rollup = MinuteRollup::default();
    samples.iter().for_each(|sample| rollup.push(sample));
    rollup.finish()
}

impl MinuteSample {
    fn empty(ts_ms: i64) -> MinuteSample {
        MinuteSample {
            ts_ms,
            samples: 0,
            cpu_avg: 0.0,
            cpu_max: 0.0,
            core_max: 0.0,
            dpc_isr_max: 0.0,
            mem_total_bytes: 0,
            mem_avail_min: u64::MAX,
            commit_max: 0,
            hard_faults_max: 0.0,
            gpu_max: None,
            perf_limit_min: f32::MAX,
            probe_max_lag_ms: 0.0,
            disks: Vec::new(),
        }
    }

    /// Folds in another summary of the same minute, weighting averages by sample count.
    pub fn merge(&mut self, other: &MinuteSample) {
        let (mine, theirs) = (f32::from(self.samples), f32::from(other.samples));
        let weighted = |a: f32, b: f32| {
            if mine + theirs > 0.0 {
                (a * mine + b * theirs) / (mine + theirs)
            } else {
                0.0
            }
        };
        self.cpu_avg = weighted(self.cpu_avg, other.cpu_avg);
        for disk in &other.disks {
            match self.disks.iter_mut().find(|d| d.name == disk.name) {
                Some(own) => {
                    own.active_max = own.active_max.max(disk.active_max);
                    own.latency_max_ms = own.latency_max_ms.max(disk.latency_max_ms);
                    own.bytes_avg = weighted(own.bytes_avg, disk.bytes_avg);
                }
                None => self.disks.push(disk.clone()),
            }
        }
        self.samples = self.samples.saturating_add(other.samples);
        self.cpu_max = self.cpu_max.max(other.cpu_max);
        self.core_max = self.core_max.max(other.core_max);
        self.dpc_isr_max = self.dpc_isr_max.max(other.dpc_isr_max);
        self.mem_total_bytes = self.mem_total_bytes.max(other.mem_total_bytes);
        self.mem_avail_min = self.mem_avail_min.min(other.mem_avail_min);
        self.commit_max = self.commit_max.max(other.commit_max);
        self.hard_faults_max = self.hard_faults_max.max(other.hard_faults_max);
        self.gpu_max = max_known(self.gpu_max, other.gpu_max);
        self.perf_limit_min = self.perf_limit_min.min(other.perf_limit_min);
        self.probe_max_lag_ms = self.probe_max_lag_ms.max(other.probe_max_lag_ms);
    }
}

/// Running summary of the samples of one minute, so nothing is buffered.
#[derive(Clone, Debug, Default)]
pub struct MinuteRollup {
    current: Option<MinuteSample>,
    cpu_sum: f64,
    /// Sum of bytes/s and sample count, one per entry of `current.disks`.
    disk_sums: Vec<(f64, u16)>,
}

impl MinuteRollup {
    /// Start of the minute being summarized, once a sample has arrived.
    pub fn minute_ms(&self) -> Option<i64> {
        self.current.as_ref().map(|minute| minute.ts_ms)
    }

    pub fn push(&mut self, sample: &Sample) {
        let minute = self
            .current
            .get_or_insert_with(|| MinuteSample::empty(minute_of(sample.ts_ms)));
        minute.samples = minute.samples.saturating_add(1);
        self.cpu_sum += f64::from(sample.cpu_pct);
        minute.cpu_max = minute.cpu_max.max(sample.cpu_pct);
        for core in &sample.cores {
            minute.core_max = minute.core_max.max(core.cpu_pct);
            minute.dpc_isr_max = minute.dpc_isr_max.max(core.dpc_pct + core.isr_pct);
        }
        minute.mem_total_bytes = minute.mem_total_bytes.max(sample.mem_total_bytes);
        minute.mem_avail_min = minute.mem_avail_min.min(sample.mem_avail_bytes);
        minute.commit_max = minute.commit_max.max(sample.commit_bytes);
        minute.hard_faults_max = minute.hard_faults_max.max(sample.hard_faults_per_s);
        minute.gpu_max = max_known(minute.gpu_max, sample.gpu_pct);
        minute.perf_limit_min = minute.perf_limit_min.min(sample.perf_limit_pct);
        minute.probe_max_lag_ms = minute.probe_max_lag_ms.max(sample.probe_max_lag_ms);
        for disk in &sample.disks {
            let at = match minute.disks.iter().position(|d| d.name == disk.name) {
                Some(at) => at,
                None => {
                    minute.disks.push(DiskMinute {
                        name: disk.name.clone(),
                        active_max: 0.0,
                        latency_max_ms: 0.0,
                        bytes_avg: 0.0,
                    });
                    self.disk_sums.push((0.0, 0));
                    minute.disks.len() - 1
                }
            };
            let own = &mut minute.disks[at];
            own.active_max = own.active_max.max(disk.active_pct);
            own.latency_max_ms = own.latency_max_ms.max(disk.latency_ms);
            let (sum, count) = &mut self.disk_sums[at];
            *sum += f64::from(disk.bytes_per_s);
            *count = count.saturating_add(1);
        }
    }

    /// The summary of what was pushed; leaves the rollup empty.
    pub fn finish(&mut self) -> Option<MinuteSample> {
        let mut minute = self.current.take()?;
        minute.cpu_avg = (self.cpu_sum / f64::from(minute.samples.max(1))) as f32;
        for (disk, (sum, count)) in minute.disks.iter_mut().zip(self.disk_sums.drain(..)) {
            disk.bytes_avg = (sum / f64::from(count.max(1))) as f32;
        }
        self.cpu_sum = 0.0;
        Some(minute)
    }
}

/// The larger of two optional readings, ignoring a missing one.
fn max_known(a: Option<f32>, b: Option<f32>) -> Option<f32> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, None) => a,
        (None, b) => b,
    }
}
