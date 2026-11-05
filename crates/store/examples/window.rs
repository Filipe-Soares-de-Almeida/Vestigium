//! What a history recorded in `[from, to]`, read-only: `window <db> <from_ms> <to_ms>`.

use vestigium_store::Store;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [db, from, to] = args.as_slice() else {
        return Err("usage: window <db> <from_ms> <to_ms>".into());
    };
    let (from, to): (i64, i64) = (from.parse()?, to.parse()?);
    let store = Store::open_read_only(std::path::Path::new(db))?;
    let samples = store.samples(from, to)?;
    let lag = samples
        .iter()
        .map(|s| s.probe_max_lag_ms)
        .fold(0.0f32, f32::max);
    let memory = samples
        .iter()
        .filter(|s| s.mem_total_bytes > 0)
        .map(|s| 100.0 * (1.0 - s.mem_avail_bytes as f64 / s.mem_total_bytes as f64))
        .fold(0.0f64, f64::max);
    let cpu = samples.iter().map(|s| s.cpu_pct).fold(0.0f32, f32::max);
    println!(
        "{} samples; probe max lag {lag:.1} ms; memory in use max {memory:.1}%; cpu max {cpu:.1}%",
        samples.len()
    );
    for e in store.episodes(from, to)? {
        println!(
            "episode {:?} {}..{} peak {}",
            e.kind, e.start_ms, e.end_ms, e.peak
        );
    }
    for o in store.occurrences(from, to)? {
        println!(
            "occurrence {:?} {:?} {}..{}",
            o.kind, o.severity, o.start_ms, o.end_ms
        );
    }
    Ok(())
}
