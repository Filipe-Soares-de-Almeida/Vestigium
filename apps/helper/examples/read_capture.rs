//! Prints a deep capture file: `read_capture <file.capture>`.

use vestigium_core::{CAPTURE_VERSION, CAPTURE_VERSION_MIN, DeepCapture, capture_findings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: read_capture <file.capture>")?;
    let bytes = std::fs::read(&path)?;
    let (version, body) = bytes.split_first().ok_or("empty file")?;
    if !(CAPTURE_VERSION_MIN..=CAPTURE_VERSION).contains(version) {
        return Err(format!(
            "capture version {version}, expected {CAPTURE_VERSION_MIN} to {CAPTURE_VERSION}"
        )
        .into());
    }
    let capture: DeepCapture = postcard::from_bytes(body)?;
    println!(
        "{} s, kernel {:?}, frames {:?}, lost {}",
        (capture.end_ms - capture.start_ms) / 1000,
        capture.kernel,
        capture.frames,
        capture.lost_events
    );
    println!("drivers (DPC+ISR):");
    for d in capture.drivers.iter().take(8) {
        println!(
            "  {:<16} dpc {:>7} max {:>5} us | isr {:>7} max {:>5} us",
            d.module, d.dpc.count, d.dpc.max_us, d.isr.count, d.isr.max_us
        );
    }
    println!("longest:");
    for r in capture.longest.iter().take(5) {
        println!(
            "  {} {:?} {} us on core {} at {}",
            r.module, r.kind, r.us, r.core, r.ts_ms
        );
    }
    println!("apps:");
    for a in capture.apps.iter().take(5) {
        println!(
            "  {} ({}) frames {} median {:.1} ms worst {:.1} ms hitches {}",
            a.name, a.pid, a.frames, a.median_ms, a.worst_ms, a.hitches
        );
    }
    println!("hitches: {}", capture.hitches.len());
    for h in &capture.hitches {
        println!(
            "  {} {} {:.1} ms (median {:.1})",
            h.ts_ms, h.app, h.frame_ms, h.median_ms
        );
    }
    println!("findings:");
    for s in capture_findings(&capture) {
        println!(
            "  {:?} {:?} ({} evidence)",
            s.confidence,
            s.cause,
            s.evidence.len()
        );
    }
    Ok(())
}
