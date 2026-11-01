//! Prints what a history database holds: `inspect <path-to-db>`.

use std::path::PathBuf;

use vestigium_store::{Series, Store};

const SERIES: [(Series, &str); 5] = [
    (Series::SamplesV1, "samples v1"),
    (Series::ProcessesV1, "processes v1"),
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
        println!("{name}: {} segments, last end_ms {last}", segments.len());
    }

    let (all, max) = (i64::MIN, i64::MAX);
    println!(
        "decoded: {} samples, {} process rankings, {} minutes",
        store.samples(all, max)?.len(),
        store.tops(all, max)?.len(),
        store.minutes(all, max)?.len()
    );
    Ok(())
}
