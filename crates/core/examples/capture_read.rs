//! Prints what a deep capture file measured and what the rules make of it: `capture_read <file>`.

use vestigium_core::{CAPTURE_VERSION, DeepCapture, capture_findings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: capture_read <file.capture>")?;
    let bytes = std::fs::read(path)?;
    let (&version, body) = bytes.split_first().ok_or("empty file")?;
    if version != CAPTURE_VERSION {
        return Err(format!("version {version}, expected {CAPTURE_VERSION}").into());
    }
    let capture: DeepCapture = postcard::from_bytes(body)?;
    println!(
        "{} s; kernel {:?}; frames {:?}; lost events {}",
        (capture.end_ms - capture.start_ms) / 1000,
        capture.kernel,
        capture.frames,
        capture.lost_events
    );
    let mut drivers = capture.drivers.clone();
    drivers.sort_by_key(|d| std::cmp::Reverse(d.dpc.max_us.max(d.isr.max_us)));
    for d in drivers.iter().take(12) {
        println!(
            "{:<20} dpc {:>8} max {:>5} us >=1ms {:>3} | isr {:>8} max {:>5} us >=1ms {:>3}",
            d.module,
            d.dpc.count,
            d.dpc.max_us,
            d.dpc.at_least(1000),
            d.isr.count,
            d.isr.max_us,
            d.isr.at_least(1000)
        );
    }
    for r in capture.longest.iter().take(8) {
        println!(
            "long {:?} {} {} us core {} at {}",
            r.kind, r.module, r.us, r.core, r.ts_ms
        );
    }
    for a in &capture.apps {
        println!(
            "app {} frames {} median {:.1} p99 {:.1} worst {:.1} hitches {}",
            a.name, a.frames, a.median_ms, a.p99_ms, a.worst_ms, a.hitches
        );
    }
    for s in capture_findings(&capture) {
        println!("finding {s:?}");
    }
    Ok(())
}
