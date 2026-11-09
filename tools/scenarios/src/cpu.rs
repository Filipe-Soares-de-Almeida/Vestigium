use std::hint::black_box;
use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::logical_cpus;

use crate::stopped;

/// Keeps every logical processor busy at normal priority for `seconds`.
pub fn burn(seconds: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    println!(
        "Keeping {} logical processors busy for {seconds} s.",
        logical_cpus()
    );
    let workers: Vec<_> = (0..logical_cpus().max(1))
        .map(|_| thread::spawn(move || spin_until(deadline)))
        .collect();
    for worker in workers {
        worker
            .join()
            .map_err(|_| "a worker thread panicked".to_string())?;
    }
    println!("Done.");
    Ok(())
}

/// Burns CPU on the calling thread until `deadline` or Ctrl+C.
pub fn spin_until(deadline: Instant) {
    let mut x = 1u64;
    while Instant::now() < deadline && !stopped() {
        for _ in 0..100_000 {
            x = black_box(x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1));
        }
    }
}
