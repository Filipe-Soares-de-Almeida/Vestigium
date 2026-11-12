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
    let mut disks: Vec<(String, f32, f32, f32, f64, usize)> = Vec::new();
    for d in samples.iter().flat_map(|s| &s.disks) {
        let i = disks
            .iter()
            .position(|(n, ..)| *n == d.name)
            .unwrap_or_else(|| {
                disks.push((d.name.clone(), 0.0, 0.0, 0.0, 0.0, 0));
                disks.len() - 1
            });
        let e = &mut disks[i];
        (e.1, e.2, e.3) = (
            e.1.max(d.active_pct),
            e.2.max(d.latency_ms.unwrap_or(0.0)),
            e.3.max(d.queue_len.unwrap_or(0.0)),
        );
        (e.4, e.5) = (e.4 + f64::from(d.active_pct), e.5 + 1);
    }
    for (name, active, latency, queue, sum, n) in disks {
        let mean = sum / n.max(1) as f64;
        println!(
            "disk {name}: active max {active:.1}% mean {mean:.1}%; latency max {latency:.1} ms; queue max {queue:.1}"
        );
    }
    for e in store.episodes(from, to)? {
        println!(
            "episode {:?} {}..{} peak {} culprit {:?}",
            e.kind, e.start_ms, e.end_ms, e.peak, e.culprit
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
