//! Private memory each 1 Hz source keeps after a few samples: `mem_cost`.

use std::thread;
use std::time::Duration;

use vestigium_collect::{CpuSampler, PdhSampler, ProcessSnapshotter, memory_info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut meter = ProcessSnapshotter::with_capacity(512 * 1024);
    let mut last = own(&mut meter)?;
    let mut step =
        |name: &str, meter: &mut ProcessSnapshotter| -> Result<(), Box<dyn std::error::Error>> {
            let now = own(meter)?;
            println!(
                "| {name} | {:+.0} KB private | {:+.0} KB working set |",
                (now.0 as f64 - last.0 as f64) / 1024.0,
                (now.1 as f64 - last.1 as f64) / 1024.0
            );
            last = now;
            Ok(())
        };
    println!("| Step | private | working set |\n|---|---|---|");
    step("baseline (meter itself)", &mut meter)?;

    let mut cpu = CpuSampler::new();
    for _ in 0..3 {
        cpu.sample()?;
        thread::sleep(Duration::from_millis(300));
    }
    step("CpuSampler, 3 samples", &mut meter)?;

    let mut pdh = PdhSampler::open()?;
    for _ in 0..3 {
        pdh.sample()?;
        thread::sleep(Duration::from_millis(300));
    }
    step("PdhSampler, 3 samples", &mut meter)?;

    for _ in 0..3 {
        memory_info()?;
    }
    step("memory_info, 3 calls", &mut meter)?;

    let mut snapshotter = ProcessSnapshotter::with_capacity(512 * 1024);
    let kept: Vec<_> = (0..6)
        .map(|_| snapshotter.snapshot())
        .collect::<Result<_, _>>()?;
    step("6 process snapshots kept", &mut meter)?;
    drop(kept);
    Ok(())
}

/// Private bytes and working set of this process.
fn own(meter: &mut ProcessSnapshotter) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let pid = std::process::id();
    let me = meter
        .snapshot()?
        .into_iter()
        .find(|p| p.pid == pid)
        .ok_or("own process not listed")?;
    Ok((me.private_bytes, me.working_set))
}
