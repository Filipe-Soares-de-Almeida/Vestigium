use std::time::{Duration, Instant};

use vestigium_collect::memory_info;

use crate::{sleep_until_stopped, stopped};

const CHUNK: usize = 64 << 20;
const PAGE: usize = 4096;
/// The commit charge never goes past this share of the commit limit.
const COMMIT_CEILING_PCT: u128 = 95;
const GIB: f64 = (1u64 << 30) as f64;

/// Bytes to allocate so free memory falls to `target_avail_pct`, never past 95% of the commit limit.
pub fn bytes_to_allocate(
    total: u64,
    avail: u64,
    commit: u64,
    commit_limit: u64,
    target_avail_pct: f64,
) -> u64 {
    let target_avail = (total as f64 * target_avail_pct / 100.0) as u64;
    let ceiling = (u128::from(commit_limit) * COMMIT_CEILING_PCT / 100) as u64;
    avail
        .saturating_sub(target_avail)
        .min(ceiling.saturating_sub(commit))
}

/// Fills memory until `avail_pct` is left, holds it for `hold_secs` touching every page, then frees it.
pub fn pressure(hold_secs: u64, avail_pct: f64) -> Result<(), String> {
    println!("The PC slows down until this ends. Ctrl+C releases everything.");
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    while !stopped() {
        let m = memory_info().map_err(|e| e.to_string())?;
        let want = bytes_to_allocate(
            m.total_bytes,
            m.avail_bytes,
            m.commit_bytes,
            m.commit_limit_bytes,
            avail_pct,
        );
        if want < CHUNK as u64 {
            break;
        }
        // A non-zero fill writes every page, so each one is really in use.
        blocks.push(vec![1u8; CHUNK]);
    }
    println!(
        "Holding {:.1} GiB for {hold_secs} s.",
        (blocks.len() * CHUNK) as f64 / GIB
    );

    let deadline = Instant::now() + Duration::from_secs(hold_secs);
    let mut stamp = 1u8;
    while Instant::now() < deadline && !stopped() {
        stamp = stamp.wrapping_add(1);
        for page in blocks.iter_mut().flat_map(|b| b.chunks_mut(PAGE)) {
            // SAFETY: `page` is a live, non-empty slice; the volatile write keeps the page in use.
            unsafe { std::ptr::write_volatile(page.as_mut_ptr(), stamp) };
        }
        sleep_until_stopped(Duration::from_secs(1));
    }
    drop(blocks);
    println!("Released.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    #[test]
    fn memory_target_is_bounded_by_commit() {
        // 32 GiB machine, 20 GiB free, target 10% free: 16.8 GiB would do, commit headroom allows 4 GiB.
        let headroom_bound = bytes_to_allocate(32 * GIB, 20 * GIB, 34 * GIB, 40 * GIB, 10.0);
        assert_eq!(headroom_bound, 38 * GIB - 34 * GIB);

        let avail_bound = bytes_to_allocate(32 * GIB, 20 * GIB, 8 * GIB, 64 * GIB, 50.0);
        assert_eq!(avail_bound, 4 * GIB);

        assert_eq!(bytes_to_allocate(32 * GIB, GIB, 8 * GIB, 64 * GIB, 7.0), 0);
        assert_eq!(
            bytes_to_allocate(32 * GIB, 20 * GIB, 63 * GIB, 64 * GIB, 7.0),
            0
        );
    }
}
