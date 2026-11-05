//! How long a history was "under strain" for the black box at several lag thresholds, read-only:
//! `strain <path-to-db>`. Strain: an open pressure episode, memory at 90% or more, or a probe lag
//! at or above the threshold within the last minute.

use std::path::PathBuf;

use vestigium_store::Store;

const FLUSH_BYTES: f64 = 4.2 * 1024.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("usage: strain <db>")?
        .into();
    let store = Store::open_read_only(&path)?;
    let samples = store.samples(i64::MIN, i64::MAX)?;
    let pressure: Vec<(i64, i64)> = store
        .episodes(i64::MIN, i64::MAX)?
        .iter()
        .filter(|e| !e.kind.is_hint())
        .map(|e| (e.start_ms, e.end_ms))
        .collect();
    let n = samples.len();
    println!(
        "{n} samples, {:.1} h, from {} to {}",
        n as f64 / 3600.0,
        samples.first().map_or(0, |s| s.ts_ms),
        samples.last().map_or(0, |s| s.ts_ms)
    );
    let mut lags: Vec<f32> = samples.iter().map(|s| s.probe_max_lag_ms).collect();
    lags.sort_by(f32::total_cmp);
    let q = |p: f64| lags[((lags.len() as f64 - 1.0) * p) as usize];
    println!(
        "probe max lag per second: p50 {:.1} p99 {:.1} p99.9 {:.1} p99.99 {:.1} max {:.1} ms",
        q(0.5),
        q(0.99),
        q(0.999),
        q(0.9999),
        lags.last().copied().unwrap_or(0.0)
    );
    for over in [8.0, 16.0, 25.0, 50.0, 100.0, 250.0] {
        let count = lags.iter().filter(|&&l| l >= over).count();
        println!("  seconds with lag >= {over:>5} ms: {count}");
    }
    let memory_high = |s: &vestigium_core::Sample| {
        s.mem_total_bytes > 0
            && (s.mem_total_bytes - s.mem_avail_bytes.min(s.mem_total_bytes)) * 100
                >= s.mem_total_bytes * 90
    };
    let in_pressure = |ts: i64| pressure.iter().any(|&(a, b)| ts >= a && ts <= b);
    println!("threshold  strain s  strain %  flushes/day  extra MB/day vs calm only");
    for threshold in [25.0f32, 50.0, 100.0, 250.0, f32::INFINITY] {
        let mut last_lag_ms = i64::MIN / 2;
        let mut strained = 0usize;
        for s in &samples {
            if s.probe_max_lag_ms >= threshold {
                last_lag_ms = s.ts_ms;
            }
            if in_pressure(s.ts_ms) || memory_high(s) || s.ts_ms - last_lag_ms <= 60_000 {
                strained += 1;
            }
        }
        let share = strained as f64 / n.max(1) as f64;
        let flushes = 86_400.0 * ((1.0 - share) / 60.0 + share / 5.0);
        let extra = (flushes - 86_400.0 / 60.0) * FLUSH_BYTES / 1e6;
        println!(
            "{threshold:>8} ms {strained:>9} {:>8.2}% {flushes:>12.0} {extra:>12.1}",
            share * 100.0
        );
    }
    Ok(())
}
