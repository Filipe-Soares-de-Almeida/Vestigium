//! Context switches each 1 Hz source adds to the calling thread: `tick_cost --ticks <n>`.

use std::time::Duration;

use vestigium_collect::{
    CpuSampler, HighResTimer, PdhSampler, ProcessSnapshotter, memory_info, monotonic_ms,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ticks: usize = std::env::args()
        .skip_while(|a| a != "--ticks")
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(20);
    let timer = HighResTimer::new()?;
    let mut snapshotter = ProcessSnapshotter::with_capacity(512 * 1024);
    let mut cpu = CpuSampler::new();
    let mut pdh = PdhSampler::open()?;
    cpu.sample()?;
    pdh.sample()?;

    println!("| Source | switches per tick | ms per call |\n|---|---|---|");
    let mut row = |name: &str, f: &mut dyn FnMut()| -> Result<(), Box<dyn std::error::Error>> {
        let before = own_switches(&mut snapshotter)?;
        let mut busy = 0.0;
        for _ in 0..ticks {
            timer.wait(Duration::from_millis(1000))?;
            let t = monotonic_ms();
            f();
            busy += monotonic_ms() - t;
        }
        let after = own_switches(&mut snapshotter)?;
        let per_tick = (after - before) as f64 / ticks as f64;
        println!("| {name} | {per_tick:.2} | {:.2} |", busy / ticks as f64);
        Ok(())
    };
    row("timer wait only", &mut || {})?;
    row("CpuSampler::sample", &mut || {
        let _ = cpu.sample();
    })?;
    row("PdhSampler::sample", &mut || {
        let _ = pdh.sample();
    })?;
    row("memory_info", &mut || {
        let _ = memory_info();
    })?;
    let mut probe = ProcessSnapshotter::with_capacity(512 * 1024);
    row("ProcessSnapshotter::snapshot", &mut || {
        let _ = probe.snapshot();
    })?;
    row("GetSystemTimes", &mut || {
        let (mut idle, mut kernel, mut user) = Default::default();
        // SAFETY: fills three local FILETIMEs.
        let _ = unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) };
    })?;
    row("GlobalMemoryStatusEx", &mut || {
        let mut status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        // SAFETY: fills a local MEMORYSTATUSEX whose length field is set.
        let _ = unsafe { GlobalMemoryStatusEx(&mut status) };
    })?;
    Ok(())
}

use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

/// Context switches of every thread of this process; the example runs on one thread.
fn own_switches(snapshotter: &mut ProcessSnapshotter) -> Result<u64, Box<dyn std::error::Error>> {
    let pid = std::process::id();
    Ok(snapshotter
        .snapshot()?
        .into_iter()
        .find(|p| p.pid == pid)
        .map_or(0, |p| p.context_switches))
}
