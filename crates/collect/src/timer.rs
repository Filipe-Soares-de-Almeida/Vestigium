use std::sync::OnceLock;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, INFINITE, SetWaitableTimer,
    TIMER_ALL_ACCESS, WaitForSingleObject,
};
use windows::core::PCWSTR;

use crate::CollectError;

/// A waitable timer precise to about a millisecond that leaves the system timer resolution alone.
#[derive(Debug)]
pub struct HighResTimer {
    handle: HANDLE,
}

// SAFETY: a timer handle is a kernel object usable from any thread.
unsafe impl Send for HighResTimer {}

impl HighResTimer {
    pub fn new() -> Result<HighResTimer, CollectError> {
        // SAFETY: an unnamed timer with default security; the handle is owned by Self.
        let handle = unsafe {
            CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            )?
        };
        Ok(HighResTimer { handle })
    }

    /// Blocks the calling thread for `due`, measured from now.
    pub fn wait(&self, due: Duration) -> Result<(), CollectError> {
        let relative_100ns = -i64::try_from(due.as_nanos() / 100).unwrap_or(i64::MAX);
        // SAFETY: one-shot relative timer with no completion routine.
        unsafe { SetWaitableTimer(self.handle, &relative_100ns, 0, None, None, false)? };
        // SAFETY: waits on the timer owned by Self.
        let result = unsafe { WaitForSingleObject(self.handle, INFINITE) };
        if result == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err(windows::core::Error::from_thread().into())
        }
    }
}

impl Drop for HighResTimer {
    fn drop(&mut self) {
        // SAFETY: closes the handle created in `new`, exactly once.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

/// Milliseconds on the performance counter, which never jumps with the wall clock.
pub fn monotonic_ms() -> f64 {
    static FREQUENCY: OnceLock<f64> = OnceLock::new();
    let frequency = *FREQUENCY.get_or_init(|| {
        let mut hz = 0i64;
        // SAFETY: writes the counter frequency into `hz`; it cannot fail on Windows XP or later.
        let _ = unsafe { QueryPerformanceFrequency(&mut hz) };
        hz as f64
    });
    let mut ticks = 0i64;
    // SAFETY: writes the current counter into `ticks`; it cannot fail on Windows XP or later.
    let _ = unsafe { QueryPerformanceCounter(&mut ticks) };
    ticks as f64 * 1000.0 / frequency
}
