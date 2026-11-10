use std::path::Path;
use std::process::Command;

use vestigium_collect::{BlackboxRing, RING_RECORDS};
use vestigium_core::{ProcessSample, ProcessTop, Sample};

/// Set in the child process that writes a ring and dies without closing it.
const CHILD: &str = "VESTIGIUM_RING_CHILD";
/// A time in 2023, after the earliest a ring record may carry and before any clock now.
const BASE: i64 = 1_700_000_000_000;

fn sample(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 50.0,
        dpc_pct: 0.0,
        isr_pct: 0.0,
        perf_limit_pct: 100.0,
        proc_perf_pct: 100.0,
        mem_total_bytes: 16 << 30,
        mem_avail_bytes: 1 << 30,
        commit_bytes: 20 << 30,
        commit_limit_bytes: 24 << 30,
        hard_faults_per_s: 0.0,
        gpu_pct: None,
        probe_max_lag_ms: 1.0,
        sampler_lag_ms: 0.0,
        cores: Vec::new(),
        disks: Vec::new(),
    }
}

fn top(ts_ms: i64) -> ProcessTop {
    ProcessTop {
        ts_ms,
        entries: vec![ProcessSample {
            pid: 42,
            name: "Browser.exe".into(),
            cpu_pct: 12.0,
            private_bytes: 14 << 30,
            read_bps: 0.0,
            write_bps: 0.0,
            gpu_pct: 0.0,
        }],
    }
}

fn write_seconds(ring: &mut BlackboxRing, from: i64, count: i64) {
    for i in 0..count {
        ring.push_sample(&sample(from + i * 1000));
    }
    ring.push_top(&top(from));
}

/// Runs as the child of `ring_survives_a_killed_process`: writes, never flushes, exits hard.
#[test]
fn ring_writer_child() {
    let Ok(path) = std::env::var(CHILD) else {
        return;
    };
    let (mut ring, _) = BlackboxRing::open(Path::new(&path)).unwrap();
    write_seconds(&mut ring, BASE, 30);
    // Ends the process with the ring still mapped and unflushed, as a kill would.
    std::process::exit(3);
}

#[test]
fn ring_survives_a_killed_process() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blackbox.ring");
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["ring_writer_child", "--exact", "--nocapture"])
        .env(CHILD, &path)
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(3),
        "the child did not die where expected"
    );

    let (_ring, held) = BlackboxRing::open(&path).unwrap();
    assert!(!held.clean);
    assert_eq!(held.samples.len(), 30);
    assert_eq!(held.samples.last().map(|s| s.ts_ms), Some(BASE + 29_000));
    assert_eq!(held.tops.len(), 1);
    assert_eq!(held.tops[0].entries[0].name, "Browser.exe");
}

#[test]
fn a_clean_close_is_read_as_clean_and_the_ring_starts_over() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blackbox.ring");
    {
        let (mut ring, held) = BlackboxRing::open(&path).unwrap();
        assert!(held.samples.is_empty());
        write_seconds(&mut ring, BASE, 10);
        ring.close_cleanly().unwrap();
    }
    let (ring, held) = BlackboxRing::open(&path).unwrap();
    assert!(held.clean);
    assert_eq!(held.samples.len(), 10);
    drop(ring);
    let (_ring, again) = BlackboxRing::open(&path).unwrap();
    assert!(
        !again.clean,
        "a ring reopened without a clean close counts as a fall"
    );
    assert!(again.samples.is_empty(), "each start begins an empty ring");
}

#[test]
fn the_ring_keeps_only_the_newest_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blackbox.ring");
    {
        let (mut ring, _) = BlackboxRing::open(&path).unwrap();
        for i in 0..RING_RECORDS as i64 + 60 {
            ring.push_sample(&sample(BASE + i * 1000));
        }
    }
    let (_ring, held) = BlackboxRing::open(&path).unwrap();
    assert_eq!(held.samples.len(), RING_RECORDS);
    assert_eq!(held.samples.first().map(|s| s.ts_ms), Some(BASE + 60_000));
    assert_eq!(
        held.samples.last().map(|s| s.ts_ms),
        Some(BASE + (RING_RECORDS as i64 + 59) * 1000)
    );
}

#[test]
fn a_cleared_ring_holds_no_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blackbox.ring");
    {
        let (mut ring, _) = BlackboxRing::open(&path).unwrap();
        write_seconds(&mut ring, BASE, 20);
        ring.clear();
        write_seconds(&mut ring, BASE + 60_000, 3);
    }
    let (_ring, held) = BlackboxRing::open(&path).unwrap();
    assert_eq!(held.samples.len(), 3);
    assert_eq!(held.samples.first().map(|s| s.ts_ms), Some(BASE + 60_000));
    assert_eq!(held.tops.len(), 1);
}

#[test]
fn a_second_start_keeps_an_unsettled_fall() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blackbox.ring");
    {
        let (mut ring, _) = BlackboxRing::open(&path).unwrap();
        write_seconds(&mut ring, BASE, 10);
    }
    {
        let (mut ring, held) = BlackboxRing::open(&path).unwrap();
        assert_eq!(held.samples.len(), 10);
        write_seconds(&mut ring, BASE + 600_000, 2);
    }

    let (ring, held) = BlackboxRing::open(&path).unwrap();
    assert!(!held.clean);
    assert_eq!(held.samples.len(), 10, "the fall not yet settled is kept");
    assert_eq!(held.samples.last().map(|s| s.ts_ms), Some(BASE + 9_000));

    ring.release_held();
    drop(ring);
    let (_ring, after) = BlackboxRing::open(&path).unwrap();
    assert!(after.samples.is_empty(), "a settled fall is not read again");
}
