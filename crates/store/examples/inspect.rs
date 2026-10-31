//! Prints what a history database holds: `inspect <path-to-db>`.

use std::path::PathBuf;

use argos_core::{ProcessTop, Sample};
use argos_store::{Series, Store, decode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("usage: inspect <path-to-db>")?
        .into();
    let store = Store::open(&path)?;
    println!("schema version {}", store.schema_version()?);

    let samples = store.segments(Series::Samples, i64::MIN, i64::MAX)?;
    let count: usize = samples
        .iter()
        .map(|s| decode::<Sample>(&s.data).map(|v| v.len()))
        .sum::<Result<_, _>>()?;
    print_series(
        "samples",
        samples.len(),
        count,
        samples.last().map(|s| s.end_ms),
    );

    let tops = store.segments(Series::Processes, i64::MIN, i64::MAX)?;
    let count: usize = tops
        .iter()
        .map(|s| decode::<ProcessTop>(&s.data).map(|v| v.len()))
        .sum::<Result<_, _>>()?;
    print_series(
        "processes",
        tops.len(),
        count,
        tops.last().map(|s| s.end_ms),
    );
    Ok(())
}

fn print_series(name: &str, segments: usize, items: usize, last_end_ms: Option<i64>) {
    let last = last_end_ms.map_or_else(|| "-".to_string(), |ms| ms.to_string());
    println!("{name}: {segments} segments, {items} items, last end_ms {last}");
}
