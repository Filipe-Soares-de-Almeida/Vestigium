use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use vestigium_core::{GateState, RunGate};
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
};

use crate::{CollectError, HighResTimer, monotonic_ms};

/// Starts the stall probe: every `period` it sends how late its tick woke up, in milliseconds.
pub fn spawn_probe(
    period: Duration,
    gate: Arc<RunGate>,
    tx: Sender<f32>,
) -> Result<JoinHandle<()>, CollectError> {
    let timer = HighResTimer::new()?;
    let period_ms = period.as_secs_f64() * 1000.0;
    let handle = thread::Builder::new()
        .name("stall-probe".to_string())
        .spawn(move || {
            // SAFETY: raises the priority of this thread only.
            let _ = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) };
            while gate.wait_while_paused() == GateState::Running {
                let armed = monotonic_ms();
                if timer.wait(period).is_err() {
                    return;
                }
                let lag = (monotonic_ms() - armed - period_ms).max(0.0);
                if tx.send(lag as f32).is_err() {
                    return;
                }
            }
        })?;
    Ok(handle)
}
