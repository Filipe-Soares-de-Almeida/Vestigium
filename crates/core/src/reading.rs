use serde::{Deserialize, Serialize};

use crate::minute::max_known;
use crate::{MinuteSample, Sample};

/// Combined load above which the PC counts as busy.
pub const BUSY_ABOVE: f32 = 38.0;
/// Share of GPU use that counts towards the load.
const GPU_WEIGHT: f32 = 0.55;
/// Memory in use counts only above this percentage.
const MEMORY_FLOOR: f32 = 35.0;

/// What the charts draw for one second, or the peaks of a longer span.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    /// Start of the span, Unix epoch milliseconds.
    pub ts_ms: i64,
    pub cpu_pct: f32,
    /// `None` when the stored layout did not record total memory.
    pub mem_used_pct: Option<f32>,
    /// Busiest physical disk.
    pub disk_active_pct: f32,
    /// Slowest physical disk.
    pub disk_latency_ms: f32,
    pub gpu_pct: Option<f32>,
    pub probe_lag_ms: f32,
}

/// How the PC fared over a span, for the day view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PcState {
    Calm,
    Busy,
    Pressure,
    /// Asleep, paused or not watched: the monitor was away and recorded it.
    Suspended,
    /// Nothing was read although the monitor was meant to be watching and recorded no absence.
    NoReading,
}

impl PcState {
    /// Every variant, in declaration order.
    pub const ALL: [PcState; 5] = [
        PcState::Calm,
        PcState::Busy,
        PcState::Pressure,
        PcState::Suspended,
        PcState::NoReading,
    ];
}

impl Reading {
    pub fn of_sample(sample: &Sample) -> Reading {
        let disks = sample.disks.iter();
        Reading {
            ts_ms: sample.ts_ms,
            cpu_pct: sample.cpu_pct,
            mem_used_pct: used_pct(sample.mem_total_bytes, sample.mem_avail_bytes),
            disk_active_pct: disks.clone().map(|d| d.active_pct).fold(0.0, f32::max),
            disk_latency_ms: disks.filter_map(|d| d.latency_ms).fold(0.0, f32::max),
            gpu_pct: sample.gpu_pct,
            probe_lag_ms: sample.probe_max_lag_ms,
        }
    }

    /// The peaks of one minute.
    pub fn of_minute(minute: &MinuteSample) -> Reading {
        let disks = minute.disks.iter();
        Reading {
            ts_ms: minute.ts_ms,
            cpu_pct: minute.cpu_max,
            mem_used_pct: used_pct(minute.mem_total_bytes, minute.mem_avail_min),
            disk_active_pct: disks.clone().map(|d| d.active_max).fold(0.0, f32::max),
            disk_latency_ms: disks.map(|d| d.latency_max_ms).fold(0.0, f32::max),
            gpu_pct: minute.gpu_max,
            probe_lag_ms: minute.probe_max_lag_ms,
        }
    }

    /// Like `of_sample`, with the disk measures read from the disk named `disk` alone.
    pub fn of_sample_on(sample: &Sample, disk: &str) -> Reading {
        let found = sample.disks.iter().find(|d| d.name == disk);
        Reading {
            disk_active_pct: found.map_or(0.0, |d| d.active_pct),
            disk_latency_ms: found.and_then(|d| d.latency_ms).unwrap_or(0.0),
            ..Reading::of_sample(sample)
        }
    }

    /// Like `of_minute`, with the disk measures read from the disk named `disk` alone.
    pub fn of_minute_on(minute: &MinuteSample, disk: &str) -> Reading {
        let found = minute.disks.iter().find(|d| d.name == disk);
        Reading {
            disk_active_pct: found.map_or(0.0, |d| d.active_max),
            disk_latency_ms: found.map_or(0.0, |d| d.latency_max_ms),
            ..Reading::of_minute(minute)
        }
    }

    /// Keeps the higher value of each measure; the start stays this one's.
    pub fn peak(&mut self, other: &Reading) {
        self.cpu_pct = self.cpu_pct.max(other.cpu_pct);
        self.mem_used_pct = max_known(self.mem_used_pct, other.mem_used_pct);
        self.disk_active_pct = self.disk_active_pct.max(other.disk_active_pct);
        self.disk_latency_ms = self.disk_latency_ms.max(other.disk_latency_ms);
        self.gpu_pct = max_known(self.gpu_pct, other.gpu_pct);
        self.probe_lag_ms = self.probe_lag_ms.max(other.probe_lag_ms);
    }

    /// The heaviest of CPU, disk, weighted GPU and memory above its floor.
    pub fn load(&self) -> f32 {
        let gpu = self.gpu_pct.unwrap_or(0.0) * GPU_WEIGHT;
        let memory = self.mem_used_pct.unwrap_or(0.0) - MEMORY_FLOOR;
        self.cpu_pct.max(self.disk_active_pct).max(gpu).max(memory)
    }
}

/// State of a span: under pressure only inside a pressure episode; with no reading it is suspended
/// when `away` says the monitor recorded its absence and no reading otherwise.
pub fn pc_state(reading: Option<&Reading>, in_episode: bool, away: bool) -> PcState {
    let Some(reading) = reading else {
        return if away {
            PcState::Suspended
        } else {
            PcState::NoReading
        };
    };
    if in_episode {
        PcState::Pressure
    } else if reading.load() > BUSY_ABOVE {
        PcState::Busy
    } else {
        PcState::Calm
    }
}

fn used_pct(total_bytes: u64, avail_bytes: u64) -> Option<f32> {
    (total_bytes > 0).then(|| {
        let used = total_bytes.saturating_sub(avail_bytes);
        (used as f64 * 100.0 / total_bytes as f64) as f32
    })
}
