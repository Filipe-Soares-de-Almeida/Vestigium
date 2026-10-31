use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// What the recording threads should be doing.
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

    fn lock(&self) -> MutexGuard<'_, GateState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
