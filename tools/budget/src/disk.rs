//! Bytes and write operations on one physical disk, read from the PDH disk counters every second.

use std::thread;
use std::time::Duration;

use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW,
    PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::core::{HSTRING, PCWSTR};

const MB: f64 = 1024.0 * 1024.0;

/// Sums what the disk named `instance` (such as "1 D:") wrote over `minutes`.
pub fn watch(instance: &str, minutes: u64) -> Result<bool, String> {
    let mut query = PDH_HQUERY::default();
    // SAFETY: opens a live-data query; closed at the end of this function.
    check(unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) })?;
    let bytes = add(
        query,
        &format!(r"\PhysicalDisk({instance})\Disk Write Bytes/sec"),
    )?;
    let writes = add(
        query,
        &format!(r"\PhysicalDisk({instance})\Disk Writes/sec"),
    )?;
    // SAFETY: the query handle is live; the first collection only primes the rates.
    check(unsafe { PdhCollectQueryData(query) })?;
    let (mut total_bytes, mut total_writes, mut busiest) = (0.0f64, 0.0f64, 0.0f64);
    for _ in 0..minutes * 60 {
        thread::sleep(Duration::from_secs(1));
        // SAFETY: the query handle is live.
        check(unsafe { PdhCollectQueryData(query) })?;
        let b = value(bytes)?;
        total_bytes += b;
        total_writes += value(writes)?;
        busiest = busiest.max(b);
    }
    // SAFETY: closes the query opened above, once.
    let _ = unsafe { PdhCloseQuery(query) };
    let seconds = (minutes * 60).max(1) as f64;
    let per_day = 86_400.0 / seconds;
    println!("| Disk {instance} | {minutes} min |\n|---|---|");
    println!("| Written | {:.2} MB |", total_bytes / MB);
    println!("| Write operations | {total_writes:.0} |");
    println!(
        "| Written per day | {:.1} MB/day |",
        total_bytes / MB * per_day
    );
    println!("| Writes per day | {:.0} |", total_writes * per_day);
    println!("| Busiest second | {:.1} KB/s |", busiest / 1024.0);
    Ok(true)
}

fn add(query: PDH_HQUERY, path: &str) -> Result<PDH_HCOUNTER, String> {
    let mut counter = PDH_HCOUNTER::default();
    // SAFETY: the query handle is live and the path outlives the call.
    check(unsafe { PdhAddEnglishCounterW(query, &HSTRING::from(path), 0, &mut counter) })
        .map_err(|e| format!("{path}: {e}"))?;
    Ok(counter)
}

fn value(counter: PDH_HCOUNTER) -> Result<f64, String> {
    let mut value = PDH_FMT_COUNTERVALUE::default();
    // SAFETY: the counter belongs to the live query; `value` outlives the call.
    check(unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut value) })?;
    // SAFETY: PDH_FMT_DOUBLE fills the double member of the union.
    Ok(unsafe { value.Anonymous.doubleValue })
}

fn check(status: u32) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("PDH status {status:#010x}"))
    }
}
