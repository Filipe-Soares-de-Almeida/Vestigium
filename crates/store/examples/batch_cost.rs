//! Writes synthetic samples the way the agent does, with a chosen batch interval and sync mode, to
//! measure disk writes: `batch_cost --dir <dir> --batch-secs <s> --minutes <m> [--full] [--warm]`.
//! `--warm` first fills the WAL past its checkpoint with past minutes, so the run measures the WAL
//! at its settled size, then waits a minute.

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, params};
use vestigium_core::{CoreLoad, DiskLoad, ProcessSample, ProcessTop, Sample};
use vestigium_store::{Batcher, Segment, Store};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(arg("--dir").ok_or("--dir is required")?);
    let batch_secs: u64 = arg("--batch-secs")
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let minutes: u64 = arg("--minutes").and_then(|s| s.parse().ok()).unwrap_or(10);
    let full = std::env::args().any(|a| a == "--full");
    let warm = std::env::args().any(|a| a == "--warm");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("batch-cost.db");
    drop(Store::open(&path)?);
    let conn = Connection::open(&path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", if full { "FULL" } else { "NORMAL" })?;
    conn.pragma_update(None, "cache_size", -256)?;
    println!(
        "batch {batch_secs} s, synchronous {}, {minutes} min",
        if full { "FULL" } else { "NORMAL" }
    );

    let mut batcher = Batcher::new(Duration::from_secs(batch_secs));
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    if warm {
        let begun = Instant::now();
        let mut past_ms = now_ms() - 30 * 86_400_000;
        for _ in 0..WARM_COMMITS {
            for s in 0..60 {
                batcher.push_sample(sample(past_ms + s * 1000, &mut seed));
                if s % 5 == 0 {
                    batcher.push_processes(top(past_ms + s * 1000, &mut seed));
                }
            }
            past_ms += 60_000;
            commit(&conn, &batcher.take())?;
        }
        println!(
            "warmed with {WARM_COMMITS} commits in {:.1} s",
            begun.elapsed().as_secs_f64()
        );
        thread::sleep(Duration::from_secs(60));
    }
    let started = Instant::now();
    let mut commits = 0u32;
    for tick in 0..minutes * 60 {
        let ts_ms = now_ms();
        batcher.push_sample(sample(ts_ms, &mut seed));
        if tick % 5 == 0 {
            batcher.push_processes(top(ts_ms, &mut seed));
        }
        if batcher.due(ts_ms) {
            commit(&conn, &batcher.take())?;
            commits += 1;
        }
        let next = Duration::from_secs(tick + 1);
        thread::sleep(next.saturating_sub(started.elapsed()));
    }
    println!(
        "{commits} commits in {:.0} s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// Past minutes written before a `--warm` run: enough to pass the 1000-page checkpoint.
const WARM_COMMITS: i64 = 1500;

fn commit(conn: &Connection, segments: &[Segment]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    for s in segments {
        tx.execute(
            "INSERT INTO segment (series, start_ms, end_ms, data) VALUES (?1, ?2, ?3, ?4)",
            params![s.series as u8, s.start_ms, s.end_ms, s.data],
        )?;
    }
    tx.commit()
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Pseudo-random value in [0, 1), so the segments compress like real readings.
fn noise(seed: &mut u64) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 40) as f32 / (1u64 << 24) as f32
}

fn sample(ts_ms: i64, seed: &mut u64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 5.0 + noise(seed) * 20.0,
        dpc_pct: noise(seed),
        isr_pct: noise(seed),
        perf_limit_pct: 100.0,
        proc_perf_pct: 90.0 + noise(seed) * 20.0,
        mem_total_bytes: 32 << 30,
        mem_avail_bytes: (16 << 30) + (noise(seed) * 1e9) as u64,
        commit_bytes: (20 << 30) + (noise(seed) * 1e9) as u64,
        commit_limit_bytes: 44 << 30,
        hard_faults_per_s: noise(seed) * 50.0,
        gpu_pct: Some(noise(seed) * 30.0),
        probe_max_lag_ms: 0.3 + noise(seed),
        sampler_lag_ms: noise(seed) * 2.0,
        cores: (0..8)
            .map(|_| CoreLoad {
                cpu_pct: noise(seed) * 40.0,
                dpc_pct: noise(seed),
                isr_pct: noise(seed),
            })
            .collect(),
        disks: ["0 C:", "1 D:"]
            .iter()
            .map(|name| DiskLoad {
                name: (*name).to_string(),
                active_pct: noise(seed) * 10.0,
                latency_ms: noise(seed) * 3.0,
                queue_len: noise(seed),
                bytes_per_s: noise(seed) * 1e6,
            })
            .collect(),
    }
}

fn top(ts_ms: i64, seed: &mut u64) -> ProcessTop {
    ProcessTop {
        ts_ms,
        entries: (0..10)
            .map(|i| ProcessSample {
                pid: 1000 + i,
                name: format!("program{i}.exe"),
                cpu_pct: noise(seed) * 10.0,
                private_bytes: (noise(seed) * 2e9) as u64,
                read_bps: noise(seed) * 1e5,
                write_bps: noise(seed) * 1e5,
                gpu_pct: noise(seed) * 5.0,
            })
            .collect(),
    }
}
