//! Prints what a history database holds, records included: `inspect <path-to-db>`.

use std::path::PathBuf;

use vestigium_core::Sample;
use vestigium_store::{Series, Store};

const SERIES: [(Series, &str); 6] = [
    (Series::SamplesV1, "samples v1"),
    (Series::ProcessesV1, "processes v1"),
    (Series::SamplesV2, "samples v2"),
    (Series::Samples, "samples"),
    (Series::Processes, "processes"),
    (Series::Minutes, "minutes"),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("usage: inspect <path-to-db>")?
        .into();
    let store = Store::open(&path)?;
    println!("schema version {}", store.schema_version()?);

    for (series, name) in SERIES {
        let segments = store.segments(series, i64::MIN, i64::MAX)?;
        let last = segments
            .last()
            .map_or_else(|| "-".to_string(), |s| s.end_ms.to_string());
        let bytes: usize = segments.iter().map(|s| s.data.len()).sum();
        let span_h = match (segments.first(), segments.last()) {
            (Some(a), Some(b)) => (b.end_ms - a.start_ms) as f64 / 3_600_000.0,
            _ => 0.0,
        };
        println!(
            "{name}: {} segments, {bytes} bytes, {:.1} KB/h over {span_h:.1} h, last end_ms {last}",
            segments.len(),
            if span_h > 0.0 {
                bytes as f64 / 1024.0 / span_h
            } else {
                0.0
            }
        );
    }

    let (all, max) = (i64::MIN, i64::MAX);
    let samples = store.samples(all, max)?;
    println!(
        "decoded: {} samples, {} process rankings, {} minutes",
        samples.len(),
        store.tops(all, max)?.len(),
        store.minutes(all, max)?.len()
    );
    let worst = |lag: fn(&Sample) -> f32| {
        samples
            .iter()
            .max_by(|a, b| lag(a).total_cmp(&lag(b)))
            .map_or_else(
                || "-".to_string(),
                |s| format!("{:.1} ms at {}", lag(s), s.ts_ms),
            )
    };
    println!("worst probe lag: {}", worst(|s| s.probe_max_lag_ms));
    println!("worst sampler lag: {}", worst(|s| s.sampler_lag_ms));

    let occurrences = store.occurrences(all, max)?;
    println!("occurrences: {}", occurrences.len());
    for o in &occurrences {
        println!(
            "  #{} {:?} {:?} at {} for {} ms, top suspect {:?}",
            o.id,
            o.kind,
            o.severity,
            o.start_ms,
            o.end_ms - o.start_ms,
            o.cause
        );
    }
    let episodes = store.episodes(all, max)?;
    println!("episodes: {}", episodes.len());
    for e in &episodes {
        println!(
            "  {:?} at {} for {} ms, peak {:.1}, culprit {:?}{}",
            e.kind,
            e.start_ms,
            e.end_ms - e.start_ms,
            e.peak,
            e.culprit.as_ref().map(|c| &c.name),
            if e.truncated { ", truncated" } else { "" }
        );
    }
    let events = store.events(all, max)?;
    println!("events: {}", events.len());
    for e in events.iter().rev().take(10) {
        println!("  {:?} {} {} at {}", e.meaning, e.provider, e.id, e.ts_ms);
    }
    let gaps = store.gaps(all, max)?;
    println!("gaps: {}", gaps.len());
    for g in &gaps {
        println!(
            "  {:?} from {} for {} ms",
            g.reason,
            g.start_ms,
            g.end_ms - g.start_ms
        );
    }
    println!("bookmark stored: {}", store.bookmark_xml()?.is_some());
    Ok(())
}
