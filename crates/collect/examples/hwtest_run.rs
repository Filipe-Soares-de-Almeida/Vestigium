//! Runs the Tests screen's quick memory test or benchmark outside the window, with a 16 ms timer
//! watching the lag meanwhile: `hwtest_run memory <minutes>` or `hwtest_run bench <C:>`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use vestigium_collect::{
    BenchTarget, HighResTimer, benchmark, disk_bus, disk_media, memory_info, memory_test,
    monotonic_ms, volume_drive,
};
use vestigium_core::Media;
use vestigium_core::hwtest::{BENCH_FILE_BYTES, BENCH_SECONDS, Bus, quick_test_size};

fn lag_probe(stop: Arc<AtomicBool>) -> thread::JoinHandle<Vec<f64>> {
    thread::spawn(move || {
        let timer = HighResTimer::new().expect("timer");
        let mut lags = Vec::new();
        while !stop.load(Ordering::Relaxed) {
            let start = monotonic_ms();
            timer.wait(Duration::from_millis(16)).expect("wait");
            lags.push(monotonic_ms() - start - 16.0);
        }
        lags
    })
}

fn lag_report(mut lags: Vec<f64>) -> String {
    lags.sort_by(f64::total_cmp);
    let at = |q: f64| lags[((lags.len() as f64 - 1.0) * q) as usize];
    format!(
        "{} ticks, p50 {:.2} ms, p99 {:.2} ms, max {:.2} ms, over 50 ms {}",
        lags.len(),
        at(0.5),
        at(0.99),
        lags.last().copied().unwrap_or(0.0),
        lags.iter().filter(|&&l| l > 50.0).count()
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stop = AtomicBool::new(false);
    let watching = Arc::new(AtomicBool::new(false));
    let probe = lag_probe(watching.clone());
    let before = memory_info().expect("memory");
    println!(
        "memory before: total {} MiB, available {} MiB",
        before.total_bytes >> 20,
        before.avail_bytes >> 20
    );
    match args.first().map(String::as_str) {
        Some("memory") => {
            let minutes: u64 = args.get(1).and_then(|m| m.parse().ok()).unwrap_or(15);
            let bytes = quick_test_size(before.total_bytes, before.avail_bytes).expect("size");
            println!("testing {} MiB for {minutes} min", bytes >> 20);
            let run = memory_test(bytes, Duration::from_secs(minutes * 60), &stop, &mut |_| {});
            println!("{run:#?}");
        }
        Some("bench") => {
            let letter = args
                .get(1)
                .expect("volume")
                .trim_end_matches('\\')
                .to_string();
            let disk = volume_drive(&letter);
            let temp = std::env::temp_dir();
            let same = temp
                .to_string_lossy()
                .to_uppercase()
                .starts_with(&letter.to_uppercase());
            let target = BenchTarget {
                dir: if same {
                    temp
                } else {
                    format!("{letter}\\").into()
                },
                volume: format!("{letter}\\"),
                disk,
                bus: disk.map_or(Bus::Other, disk_bus),
                media: disk.map_or(Media::Unknown, disk_media),
            };
            println!("{target:?}");
            let run = benchmark(
                &target,
                BENCH_FILE_BYTES,
                BENCH_SECONDS,
                &stop,
                &mut |mode| println!("  {mode:?} at {:.0} s", monotonic_ms() / 1000.0),
            );
            println!("{run:#?}");
        }
        _ => eprintln!("hwtest_run memory <minutes> | bench <C:>"),
    }
    watching.store(true, Ordering::Relaxed);
    let lags = probe.join().expect("probe");
    let after = memory_info().expect("memory");
    println!("memory after: available {} MiB", after.avail_bytes >> 20);
    println!("timer lag: {}", lag_report(lags));
}
