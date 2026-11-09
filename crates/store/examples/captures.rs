//! Lists the deep captures in a database and what each adds to the occurrences it covers:
//! `captures <vestigium.db>`.

use std::path::PathBuf;

use vestigium_core::capture_suspects;
use vestigium_store::Store;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args_os()
        .nth(1)
        .ok_or("usage: captures <vestigium.db>")?
        .into();
    let store = Store::open_read_only(&path)?;
    for summary in store.captures(i64::MIN, i64::MAX)? {
        let Some(capture) = store.capture(summary.id)? else {
            continue;
        };
        let (dpc, isr): (u64, u64) = capture
            .drivers
            .iter()
            .fold((0, 0), |(d, i), x| (d + x.dpc.count, i + x.isr.count));
        println!(
            "capture {} from {} to {}: {} DPC, {} ISR, {} lost, {} hitches, {} probe hiccups",
            summary.id,
            summary.start_ms,
            summary.end_ms,
            dpc,
            isr,
            capture.lost_events,
            capture.hitches.len(),
            capture.hiccups.len()
        );
        for occurrence in store.occurrences(summary.start_ms, summary.end_ms)? {
            let Some(full) = store.occurrence(occurrence.id)? else {
                continue;
            };
            let (from, to) = (full.window.from_ms, full.window.to_ms);
            println!("  occurrence {} ({:?}):", occurrence.id, occurrence.kind);
            for suspect in capture_suspects(from, to, &capture) {
                println!(
                    "    {:?} {:?} with {} evidence",
                    suspect.confidence,
                    suspect.cause,
                    suspect.evidence.len()
                );
            }
        }
    }
    Ok(())
}
