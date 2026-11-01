//! Lists GPU use per process every 5 s from one query opened at start, marking processes that
//! appear later, and the media of each disk: `gpu_watch --seconds <s>`.

use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

use vestigium_collect::{PdhSampler, ProcessSnapshotter, disk_media, physical_drive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds: u64 = std::env::args()
        .skip_while(|a| a != "--seconds")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let mut pdh = PdhSampler::open()?;
    let mut processes = ProcessSnapshotter::with_capacity(512 * 1024);
    pdh.sample()?;
    thread::sleep(Duration::from_secs(1));
    let first = pdh.sample()?.ok_or("no reading after the priming one")?;

    for disk in &first.disks {
        let media = physical_drive(&disk.name).map(disk_media);
        println!("disk {}: {media:?}", disk.name);
    }
    let Some(gpu) = first.gpu else {
        println!("no GPU Engine counters on this machine");
        return Ok(());
    };
    let mut seen: HashSet<u32> = gpu.per_pid.keys().copied().collect();
    println!(
        "{} processes on the GPU at start, busiest engine {:.1}%",
        seen.len(),
        gpu.system_pct
    );

    for elapsed in (5..=seconds).step_by(5) {
        thread::sleep(Duration::from_secs(5));
        let Some(gpu) = pdh.sample()?.and_then(|r| r.gpu) else {
            println!("{elapsed:>4} s  no GPU reading");
            continue;
        };
        let names: HashMap<u32, String> = processes
            .snapshot()?
            .into_iter()
            .map(|p| (p.pid, p.name))
            .collect();
        let new: Vec<String> = gpu
            .per_pid
            .iter()
            .filter(|(pid, _)| seen.insert(**pid))
            .map(|(pid, pct)| {
                let name = names.get(pid).map_or("?", String::as_str);
                format!("{name} ({pid}) {pct:.1}%")
            })
            .collect();
        println!(
            "{elapsed:>4} s  {} processes, busiest engine {:.1}%, new: {}",
            gpu.per_pid.len(),
            gpu.system_pct,
            if new.is_empty() {
                "-".to_string()
            } else {
                new.join(", ")
            }
        );
    }
    Ok(())
}
