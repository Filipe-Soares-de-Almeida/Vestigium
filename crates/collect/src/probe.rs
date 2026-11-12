use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
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
    dropped: AtomicU64,
}

impl ProbeConfig {
    pub fn new(period: Duration) -> Arc<ProbeConfig> {
        Arc::new(ProbeConfig {
            period_us: AtomicU32::new(micros(period)),
            dropped: AtomicU64::new(0),
        })
    }

    /// Ticks the probe dropped because the queue to the sampler was full.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
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
    tx: SyncSender<ProbeTick>,
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
                if !offer(&tx, tick, &config) {
                    return;
                }
            }
        })?;
    Ok(handle)
}

/// Queues a tick without ever blocking the probe; a full queue drops it and counts it. False
/// once the sampler is gone.
fn offer(tx: &SyncSender<ProbeTick>, tick: ProbeTick, config: &ProbeConfig) -> bool {
    match tx.try_send(tick) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) => {
            config.dropped.fetch_add(1, Ordering::Relaxed);
            true
        }
        Err(TrySendError::Disconnected(_)) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn tick(wall_ms: i64) -> ProbeTick {
        ProbeTick {
            wall_ms,
            lag_ms: 0.0,
        }
    }

    #[test]
    fn a_full_queue_drops_and_counts_ticks_instead_of_blocking() {
        let config = ProbeConfig::new(Duration::from_millis(100));
        let (tx, rx) = mpsc::sync_channel(2);

        for at in 0..5 {
            assert!(offer(&tx, tick(at), &config));
        }

        assert_eq!(config.dropped(), 3);
        assert_eq!(rx.try_iter().map(|t| t.wall_ms).collect::<Vec<_>>(), [0, 1]);
    }

    #[test]
    fn a_gone_sampler_ends_the_probe() {
        let config = ProbeConfig::new(Duration::from_millis(100));
        let (tx, rx) = mpsc::sync_channel(2);
        drop(rx);

        assert!(!offer(&tx, tick(0), &config));
    }
}
