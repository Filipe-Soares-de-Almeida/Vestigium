use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_EVENT, WAIT_OBJECT_0};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW, INFINITE,
    SetEvent, SetWaitableTimer, TIMER_ALL_ACCESS, WaitForMultipleObjects, WaitForSingleObject,
};
use windows::Win32::System::WindowsProgramming::{
    QueryInterruptTimePrecise, QueryUnbiasedInterruptTimePrecise,
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

/// An event that any thread sets to end a wait on the timer early; it resets itself when a
/// wait takes it.
#[derive(Debug)]
pub struct Wake {
    handle: HANDLE,
}

// SAFETY: an event handle is a kernel object usable from any thread.
unsafe impl Send for Wake {}
// SAFETY: setting an event is safe from any thread at the same time.
unsafe impl Sync for Wake {}

impl Wake {
    pub fn new() -> Result<Wake, CollectError> {
        // SAFETY: an unnamed auto-reset event, initially clear; the handle is owned by Self.
        let handle = unsafe { CreateEventW(None, false, false, PCWSTR::null())? };
        Ok(Wake { handle })
    }

    /// Ends the wait in progress, or the next one if none is under way.
    pub fn set(&self) {
        // SAFETY: signals the event owned by Self.
        let _ = unsafe { SetEvent(self.handle) };
    }
}

impl Drop for Wake {
    fn drop(&mut self) {
        // SAFETY: closes the handle created in `new`, exactly once.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

/// How a wait on the timer ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waited {
    /// The time passed.
    Elapsed,
    /// The wake event was set first.
    Woken,
}

impl HighResTimer {
    /// Blocks for `due` like `wait`, or until `wake` is set, and says which came first.
    pub fn wait_or_wake(&self, due: Duration, wake: &Wake) -> Result<Waited, CollectError> {
        let relative_100ns = -i64::try_from(due.as_nanos() / 100).unwrap_or(i64::MAX);
        // SAFETY: one-shot relative timer with no completion routine.
        unsafe { SetWaitableTimer(self.handle, &relative_100ns, 0, None, None, false)? };
        // SAFETY: waits on the timer and the event, both owned for the whole call.
        let result =
            unsafe { WaitForMultipleObjects(&[self.handle, wake.handle], false, INFINITE) };
        if result == WAIT_OBJECT_0 {
            Ok(Waited::Elapsed)
        } else if result == WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
            Ok(Waited::Woken)
        } else {
            Err(windows::core::Error::from_thread().into())
        }
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

/// Milliseconds of interrupt time, leaving out the time the system spent asleep or hibernating.
pub fn unbiased_ms() -> f64 {
    // SAFETY: a plain read of the interrupt-time counter.
    let ticks_100ns = unsafe { QueryUnbiasedInterruptTimePrecise() };
    ticks_100ns as f64 / 10_000.0
}

/// Milliseconds of interrupt time since boot, counting the time spent asleep or hibernating and
/// never moving with the wall clock: the clock for deadlines that must survive both.
pub fn interrupt_ms() -> i64 {
    // SAFETY: a plain read of the interrupt-time counter.
    let ticks_100ns = unsafe { QueryInterruptTimePrecise() };
    i64::try_from(ticks_100ns / 10_000).unwrap_or(i64::MAX)
}

/// The wall clock, Unix epoch milliseconds.
pub fn wall_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use super::*;

    #[test]
    fn interrupt_time_runs_forward_and_is_not_the_wall_clock() {
        let first = interrupt_ms();
        std::thread::sleep(Duration::from_millis(30));
        let second = interrupt_ms();
        assert!(second - first >= 25, "{first} then {second}");
        // Counted from boot, so far below an epoch time, and never behind the unbiased clock.
        assert!(second < wall_ms());
        assert!(second as f64 >= unbiased_ms() - 2.0);
    }

    #[test]
    fn a_wait_with_no_wake_lasts_its_time() {
        let (timer, wake) = (HighResTimer::new().unwrap(), Wake::new().unwrap());
        let began = Instant::now();
        let waited = timer
            .wait_or_wake(Duration::from_millis(60), &wake)
            .unwrap();
        assert_eq!(waited, Waited::Elapsed);
        assert!(began.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn a_wake_ends_a_long_wait_at_once() {
        let (timer, wake) = (HighResTimer::new().unwrap(), Arc::new(Wake::new().unwrap()));
        let setter = {
            let wake = wake.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(40));
                wake.set();
            })
        };
        let began = Instant::now();
        let waited = timer.wait_or_wake(Duration::from_secs(30), &wake).unwrap();
        setter.join().unwrap();
        assert_eq!(waited, Waited::Woken);
        assert!(began.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_wake_set_before_the_wait_is_not_lost_and_is_taken_once() {
        let (timer, wake) = (HighResTimer::new().unwrap(), Wake::new().unwrap());
        wake.set();
        assert_eq!(
            timer.wait_or_wake(Duration::from_secs(30), &wake).unwrap(),
            Waited::Woken
        );
        assert_eq!(
            timer
                .wait_or_wake(Duration::from_millis(30), &wake)
                .unwrap(),
            Waited::Elapsed
        );
    }
}
