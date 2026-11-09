use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::logical_cpus;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, GetPriorityClass, HIGH_PRIORITY_CLASS,
    NORMAL_PRIORITY_CLASS, OpenProcessToken, REALTIME_PRIORITY_CLASS, SetPriorityClass,
    SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
};

use crate::cpu::spin_until;

/// Time to let go of the mouse before the freeze starts.
const COUNTDOWN: Duration = Duration::from_secs(3);
/// Time alive after the freeze, so the watcher's next process snapshots still find this process.
const LINGER: Duration = Duration::from_secs(2);

/// One real-time busy thread per logical processor, which stops every normal thread, the stall probe included.
pub fn starve(seconds: u64) -> Result<(), String> {
    if !is_elevated()? {
        return Err(
            "starve needs an elevated terminal: without it Windows lowers real-time \
                    priority to high, and the stall probe keeps running"
                .to_string(),
        );
    }
    println!(
        "The PC freezes for {seconds} s, starting in {} s.",
        COUNTDOWN.as_secs()
    );
    thread::sleep(COUNTDOWN);
    // SAFETY: changes the priority class of this process only.
    unsafe { SetPriorityClass(GetCurrentProcess(), REALTIME_PRIORITY_CLASS) }
        .map_err(|e| e.to_string())?;
    // SAFETY: a plain query on this process.
    let class = unsafe { GetPriorityClass(GetCurrentProcess()) };
    let result = if class == REALTIME_PRIORITY_CLASS.0 {
        run(seconds)
    } else {
        Err(format!(
            "Windows kept priority class {class:#x}; nothing was run"
        ))
    };
    // SAFETY: restores this process's own priority class.
    let _ = unsafe { SetPriorityClass(GetCurrentProcess(), NORMAL_PRIORITY_CLASS) };
    if result.is_ok() {
        thread::sleep(LINGER);
    }
    result
}

/// `bursts` busy stalls of `burst_ms` each, `every_ms` apart: hitches the probe sees, short of a freeze.
pub fn stutter(bursts: u64, every_ms: u64, burst_ms: u64) -> Result<(), String> {
    // Without elevation the workers share priority 15 with the probe, which then waits its turn.
    let class = if is_elevated()? {
        REALTIME_PRIORITY_CLASS
    } else {
        HIGH_PRIORITY_CLASS
    };
    println!(
        "{bursts} stalls of {burst_ms} ms, one every {every_ms} ms, starting in {} s.",
        COUNTDOWN.as_secs()
    );
    thread::sleep(COUNTDOWN);
    // SAFETY: changes the priority class of this process only.
    unsafe { SetPriorityClass(GetCurrentProcess(), class) }.map_err(|e| e.to_string())?;
    let mut result = Ok(());
    for _ in 0..bursts {
        if crate::stopped() {
            break;
        }
        let started = Instant::now();
        result = stall(Instant::now() + Duration::from_millis(burst_ms));
        if result.is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(every_ms).saturating_sub(started.elapsed()));
    }
    // SAFETY: restores this process's own priority class.
    let _ = unsafe { SetPriorityClass(GetCurrentProcess(), NORMAL_PRIORITY_CLASS) };
    if result.is_ok() {
        println!("Done: {bursts} stalls.");
        thread::sleep(LINGER);
    }
    result
}

fn run(seconds: u64) -> Result<(), String> {
    stall(Instant::now() + Duration::from_secs(seconds))?;
    println!("Back after {seconds} s.");
    Ok(())
}

fn stall(deadline: Instant) -> Result<(), String> {
    let workers: Vec<_> = (0..logical_cpus().max(1))
        .map(|_| {
            thread::spawn(move || {
                // SAFETY: raises the priority of this worker thread only.
                let _ =
                    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) };
                spin_until(deadline);
            })
        })
        .collect();
    for worker in workers {
        worker
            .join()
            .map_err(|_| "a worker thread panicked".to_string())?;
    }
    Ok(())
}

fn is_elevated() -> Result<bool, String> {
    let mut token = HANDLE::default();
    // SAFETY: opens this process's token for query into `token`, closed below.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|e| e.to_string())?;
    let mut elevation = TOKEN_ELEVATION::default();
    let mut len = 0u32;
    // SAFETY: `elevation` has room for the TOKEN_ELEVATION the call writes.
    let queried = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&raw mut elevation).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
    };
    // SAFETY: `token` was opened above and is closed exactly once.
    let _ = unsafe { CloseHandle(token) };
    queried.map_err(|e| e.to_string())?;
    Ok(elevation.TokenIsElevated != 0)
}
