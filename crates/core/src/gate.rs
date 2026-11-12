use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// What the watcher threads should be doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateState {
    Running,
    Paused,
    Stopping,
}

/// Pause and stop signal shared by the probe and the sampler.
#[derive(Debug)]
pub struct RunGate {
    state: Mutex<GateState>,
    changed: Condvar,
}

impl RunGate {
    pub fn new() -> Arc<RunGate> {
        Arc::new(RunGate {
            state: Mutex::new(GateState::Running),
            changed: Condvar::new(),
        })
    }

    pub fn set(&self, state: GateState) {
        *self.lock() = state;
        self.changed.notify_all();
    }

    pub fn state(&self) -> GateState {
        *self.lock()
    }

    /// Blocks while paused; returns `Running` or `Stopping`.
    pub fn wait_while_paused(&self) -> GateState {
        let guard = self
            .changed
            .wait_while(self.lock(), |state| *state == GateState::Paused)
            .unwrap_or_else(PoisonError::into_inner);
        *guard
    }

    /// Blocks while paused, for at most `timeout`; returns the state it ends in.
    pub fn wait_while_paused_for(&self, timeout: Duration) -> GateState {
        let (guard, _) = self
            .changed
            .wait_timeout_while(self.lock(), timeout, |state| *state == GateState::Paused)
            .unwrap_or_else(PoisonError::into_inner);
        *guard
    }

    /// Sleeps while running, for at most `timeout`; a pause or a stop ends the wait at once.
    pub fn wait_while_running_for(&self, timeout: Duration) -> GateState {
        let (guard, _) = self
            .changed
            .wait_timeout_while(self.lock(), timeout, |state| *state == GateState::Running)
            .unwrap_or_else(PoisonError::into_inner);
        *guard
    }

    fn lock(&self) -> MutexGuard<'_, GateState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
