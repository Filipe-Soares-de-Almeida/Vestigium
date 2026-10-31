use std::ffi::c_void;

use vestigium_core::CoreLoad;
use windows::Wdk::System::SystemInformation::{
    NtQuerySystemInformation, SystemProcessorPerformanceInformation,
};

use crate::{CollectError, logical_cpus};

/// x64 layout of SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION, one per logical processor.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct ProcessorTimes {
    idle: i64,
    kernel: i64,
    user: i64,
    dpc: i64,
    interrupt: i64,
    interrupt_count: u32,
}

const _: () = assert!(size_of::<ProcessorTimes>() == 48);

/// Cumulative times of one logical processor, in 100 ns units; `kernel` includes `idle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoreTimes {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
    pub dpc: u64,
    pub interrupt: u64,
}

/// CPU use of one interval, system-wide and per logical processor.
#[derive(Clone, Debug, PartialEq)]
pub struct CpuReading {
    pub cpu_pct: f32,
    pub dpc_pct: f32,
    pub isr_pct: f32,
    pub cores: Vec<CoreLoad>,
}

/// Busy, DPC and interrupt shares of each processor between two readings.
pub fn core_loads(prev: &[CoreTimes], curr: &[CoreTimes]) -> Vec<CoreLoad> {
    prev.iter()
        .zip(curr)
        .map(|(a, b)| load(&delta(a, b)))
        .collect()
}

fn delta(a: &CoreTimes, b: &CoreTimes) -> CoreTimes {
    CoreTimes {
        idle: b.idle.saturating_sub(a.idle),
        kernel: b.kernel.saturating_sub(a.kernel),
        user: b.user.saturating_sub(a.user),
        dpc: b.dpc.saturating_sub(a.dpc),
        interrupt: b.interrupt.saturating_sub(a.interrupt),
    }
}

fn load(d: &CoreTimes) -> CoreLoad {
    let total = (d.kernel + d.user) as f64;
    let share = |part: u64| {
        if total > 0.0 {
            (part as f64 / total * 100.0).clamp(0.0, 100.0) as f32
        } else {
            0.0
        }
    };
    CoreLoad {
        cpu_pct: share((d.kernel + d.user).saturating_sub(d.idle)),
        dpc_pct: share(d.dpc),
        isr_pct: share(d.interrupt),
    }
}

/// Reads processor times with one NtQuerySystemInformation call; covers the calling thread's processor group.
#[derive(Debug, Default)]
pub struct CpuSampler {
    prev: Option<Vec<CoreTimes>>,
    buf: Vec<ProcessorTimes>,
}

impl CpuSampler {
    pub fn new() -> CpuSampler {
        CpuSampler {
            prev: None,
            buf: vec![ProcessorTimes::default(); logical_cpus().clamp(1, 64) as usize],
        }
    }

    /// `None` on the first reading after `new` or `reset`, which has nothing to compare with.
    pub fn sample(&mut self) -> Result<Option<CpuReading>, CollectError> {
        let curr = self.read()?;
        let Some(prev) = self.prev.replace(curr.clone()) else {
            return Ok(None);
        };
        let cores = core_loads(&prev, &curr);
        let sum = |times: &[CoreTimes]| {
            times.iter().fold(CoreTimes::default(), |acc, t| CoreTimes {
                idle: acc.idle + t.idle,
                kernel: acc.kernel + t.kernel,
                user: acc.user + t.user,
                dpc: acc.dpc + t.dpc,
                interrupt: acc.interrupt + t.interrupt,
            })
        };
        let total = load(&delta(&sum(&prev), &sum(&curr)));
        Ok(Some(CpuReading {
            cpu_pct: total.cpu_pct,
            dpc_pct: total.dpc_pct,
            isr_pct: total.isr_pct,
            cores,
        }))
    }

    pub fn reset(&mut self) {
        self.prev = None;
    }

    fn read(&mut self) -> Result<Vec<CoreTimes>, CollectError> {
        let capacity =
            u32::try_from(self.buf.len() * size_of::<ProcessorTimes>()).unwrap_or(u32::MAX);
        let mut written = 0u32;
        // SAFETY: the buffer holds `capacity` writable bytes of correctly aligned entries.
        unsafe {
            NtQuerySystemInformation(
                SystemProcessorPerformanceInformation,
                self.buf.as_mut_ptr().cast::<c_void>(),
                capacity,
                &mut written,
            )
            .ok()?
        };
        let count = written as usize / size_of::<ProcessorTimes>();
        let as_u64 = |v: i64| v.max(0) as u64;
        Ok(self.buf[..count.min(self.buf.len())]
            .iter()
            .map(|t| CoreTimes {
                idle: as_u64(t.idle),
                kernel: as_u64(t.kernel),
                user: as_u64(t.user),
                dpc: as_u64(t.dpc),
                interrupt: as_u64(t.interrupt),
            })
            .collect())
    }
}
