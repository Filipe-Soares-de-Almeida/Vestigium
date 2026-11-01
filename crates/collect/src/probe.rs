use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use vestigium_core::{GateState, ProbeTick, RunGate};
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
};

use crate::{CollectError, HighResTimer, unbiased_ms, wall_ms};

/// The probe period, which can change while the probe runs.
#[derive(Debug)]
pub struct ProbeConfig {
    period_us: AtomicU32,
}

impl ProbeConfig {
    pub fn new(period: Duration) -> Arc<ProbeConfig> {
        Arc::new(ProbeConfig {
            period_us: AtomicU32::new(micros(period)),
        })
    }

    /// Takes effect from the probe's next tick.
    pub fn set(&self, period: Duration) {
        self.period_us.store(micros(period), Ordering::Relaxed);
    }

    pub fn period(&self) -> Duration {
        Duration::from_micros(u64::from(self.period_us.load(Ordering::Relaxed)))
    }
}

fn micros(period: Duration) -> u32 {
    u32::try_from(period.as_micros()).unwrap_or(u32::MAX).max(1)
}

/// Starts the stall probe: every period of `config` it sends when its tick woke up and how late.
pub fn spawn_probe(
    config: Arc<ProbeConfig>,
    gate: Arc<RunGate>,
    tx: Sender<ProbeTick>,
) -> Result<JoinHandle<()>, CollectError> {
    let timer = HighResTimer::new()?;
    let handle = thread::Builder::new()
        .name("stall-probe".to_string())
        .spawn(move || {
            // SAFETY: raises the priority of this thread only.
            let _ = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) };
            while gate.wait_while_paused() == GateState::Running {
                let period = config.period();
                let period_ms = period.as_secs_f64() * 1000.0;
                let armed = unbiased_ms();
                if timer.wait(period).is_err() {
                    return;
                }
                let lag = (unbiased_ms() - armed - period_ms).max(0.0);
                let tick = ProbeTick {
                    wall_ms: wall_ms(),
                    lag_ms: lag as f32,
                };
                if tx.send(tick).is_err() {
                    return;
                }
            }
        })?;
    Ok(handle)
}
