//! Measures the stall probe: `probe_precision --period-ms <n> --seconds <s>`.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use vestigium_collect::{ProbeConfig, spawn_probe};
use vestigium_core::{GateState, RunGate};
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let period_ms = arg("--period-ms").unwrap_or(100);
    let seconds = arg("--seconds").unwrap_or(60);

    let gate = RunGate::new();
    let (tx, rx) = mpsc::sync_channel(4096);
    let cpu_before = process_cpu_ms();
    let probe = spawn_probe(
        ProbeConfig::new(Duration::from_millis(period_ms)),
        gate.clone(),
        tx,
    )?;
    thread::sleep(Duration::from_secs(seconds));
    gate.set(GateState::Stopping);
    probe.join().map_err(|_| "probe thread panicked")?;
    let cpu_ms = process_cpu_ms() - cpu_before;

    let mut lags: Vec<f32> = rx.try_iter().map(|tick| tick.lag_ms).collect();
    lags.sort_by(f32::total_cmp);
    let pct = |p: f64| lags[((lags.len() - 1) as f64 * p).round() as usize];
    println!(
        "| period | seconds | ticks | ticks/s | p50 ms | p99 ms | p99.9 ms | max ms | CPU ms |"
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    println!(
        "| {period_ms} ms | {seconds} | {} | {:.2} | {:.3} | {:.3} | {:.3} | {:.3} | {cpu_ms:.0} |",
        lags.len(),
        lags.len() as f64 / seconds as f64,
        pct(0.5),
        pct(0.99),
        pct(0.999),
        pct(1.0),
    );
    Ok(())
}

fn arg(name: &str) -> Option<u64> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1)?.parse().ok()
}

fn process_cpu_ms() -> f64 {
    let (mut created, mut exited, mut kernel, mut user) = Default::default();
    // SAFETY: queries the current process into four local FILETIMEs.
    let _ = unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    };
    (filetime_100ns(kernel) + filetime_100ns(user)) as f64 / 10_000.0
}

fn filetime_100ns(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}
