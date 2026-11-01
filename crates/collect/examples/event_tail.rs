//! Lists the curated Windows events of the last `--days` days (7 by default), then resumes from
//! the last bookmark to show nothing is read twice: `event_tail [--days <d>]`.

use std::sync::mpsc;
use std::time::Duration;

use vestigium_collect::{Delivered, EventSubscription};

const QUIET: Duration = Duration::from_secs(3);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let days: u64 = std::env::args()
        .skip_while(|a| a != "--days")
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let (tx, rx) = mpsc::channel();
    let subscription = EventSubscription::start(None, Duration::from_secs(days * 86_400), tx)?;
    let events: Vec<Delivered> = std::iter::from_fn(|| rx.recv_timeout(QUIET).ok()).collect();
    drop(subscription);

    for Delivered { event, .. } in &events {
        println!(
            "{}  {:<12} {:<45} {:>5}  {:?}",
            event.ts_ms,
            event.channel.path(),
            event.provider,
            event.id,
            event.meaning
        );
    }
    println!("{} curated events in {days} days", events.len());

    let Some(last) = events.last() else {
        return Ok(());
    };
    let (tx, rx) = mpsc::channel();
    let _resumed = EventSubscription::start(Some(&last.bookmark_xml), Duration::ZERO, tx)?;
    let replayed = std::iter::from_fn(|| rx.recv_timeout(QUIET).ok()).count();
    println!("resumed after the last one: {replayed} events read again");
    Ok(())
}
