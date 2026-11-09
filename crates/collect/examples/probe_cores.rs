//! Probe lag per core and short DPC bursts: `probe_cores --seconds <s> [--period-ms 16] [--pin <core>|--rotate|--dpc]`.

use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::{CpuSampler, HighResTimer, logical_cpus, unbiased_ms};
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadAffinityMask, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds: u64 = arg("--seconds").and_then(|s| s.parse().ok()).unwrap_or(60);
    let period_ms: u64 = arg("--period-ms")
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);
    let cpus = logical_cpus().clamp(1, 64) as usize;
    if std::env::args().any(|a| a == "--dpc") {
        return dpc_windows(seconds);
    }
    let pin: Option<usize> = arg("--pin").and_then(|s| s.parse().ok());
    let rotate = std::env::args().any(|a| a == "--rotate");
    let handle = thread::spawn(move || -> Vec<(usize, f64)> {
        // SAFETY: raises this thread's own priority, as the agent's probe does.
        let _ = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) };
        let timer = HighResTimer::new().expect("timer");
        let period = Duration::from_millis(period_ms);
        let deadline = Instant::now() + Duration::from_secs(seconds);
        let mut lags = Vec::new();
        let mut i = 0usize;
        while Instant::now() < deadline {
            let core = match (pin, rotate) {
                (Some(k), _) => k,
                (None, true) => i % cpus,
                _ => usize::MAX,
            };
            if core != usize::MAX {
                // SAFETY: restricts this thread to one existing logical processor.
                unsafe { SetThreadAffinityMask(GetCurrentThread(), 1usize << core) };
            }
            let before = unbiased_ms();
            timer.wait(period).expect("wait");
            let lag = unbiased_ms() - before - period_ms as f64;
            lags.push((core, lag.max(0.0)));
            i += 1;
        }
        lags
    });
    let lags = handle.join().map_err(|_| "probe panicked")?;
    let mode = match (pin, rotate) {
        (Some(k), _) => format!("pinned to {k}"),
        (None, true) => "rotating".into(),
        _ => "free".into(),
    };
    println!("| mode | core | ticks | p50 ms | p99 ms | p99.9 ms | max ms | >= 4 ms | >= 8 ms |");
    println!("|---|---|---|---|---|---|---|---|---|");
    let mut groups: Vec<usize> = lags.iter().map(|l| l.0).collect();
    groups.sort();
    groups.dedup();
    for core in groups {
        let mut v: Vec<f64> = lags.iter().filter(|l| l.0 == core).map(|l| l.1).collect();
        v.sort_by(f64::total_cmp);
        let label = if core == usize::MAX {
            "any".to_string()
        } else {
            core.to_string()
        };
        println!(
            "| {mode} | {label} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {} | {} |",
            v.len(),
            pct(&v, 0.5),
            pct(&v, 0.99),
            pct(&v, 0.999),
            pct(&v, 1.0),
            v.iter().filter(|x| **x >= 4.0).count(),
            v.iter().filter(|x| **x >= 8.0).count()
        );
    }
    Ok(())
}

/// Per-core DPC + ISR share at 100 ms and at 1 s, and what reading it every 100 ms costs.
fn dpc_windows(seconds: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut sampler = CpuSampler::new();
    let _ = sampler.sample()?;
    let cpus = logical_cpus().clamp(1, 64) as usize;
    let mut tenth: Vec<Vec<f32>> = vec![Vec::new(); cpus];
    let mut cost_us = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
        let t = Instant::now();
        let reading = sampler.sample()?;
        cost_us.push(t.elapsed().as_secs_f64() * 1e6);
        if let Some(r) = reading {
            for (core, load) in r.cores.iter().enumerate().take(cpus) {
                tenth[core].push(load.dpc_pct + load.isr_pct);
            }
        }
    }
    cost_us.sort_by(f64::total_cmp);
    println!(
        "reads: {}, cost p50 {:.1} us, p99 {:.1} us, max {:.1} us",
        cost_us.len(),
        pct(&cost_us, 0.5),
        pct(&cost_us, 0.99),
        pct(&cost_us, 1.0)
    );
    println!(
        "| core | max DPC+ISR % in 100 ms | max in 1 s (mean of 10) | 100 ms windows >= 10% | >= 25% | 1 s windows >= 10% |"
    );
    println!("|---|---|---|---|---|---|");
    for (core, v) in tenth.iter().enumerate() {
        let seconds: Vec<f32> = v
            .chunks(10)
            .map(|c| c.iter().sum::<f32>() / c.len() as f32)
            .collect();
        println!(
            "| {core} | {:.1} | {:.1} | {} | {} | {} |",
            v.iter().cloned().fold(0.0, f32::max),
            seconds.iter().cloned().fold(0.0, f32::max),
            v.iter().filter(|x| **x >= 10.0).count(),
            v.iter().filter(|x| **x >= 25.0).count(),
            seconds.iter().filter(|x| **x >= 10.0).count()
        );
    }
    Ok(())
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1).cloned()
}
