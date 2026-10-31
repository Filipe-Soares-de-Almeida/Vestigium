use std::thread;
use std::time::{Duration, Instant};

use watchmypc_collect::{
    CpuInstance, PdhSampler, ProcessSnapshotter, logical_cpus, memory_info, parse_cpu_instance,
};

#[test]
fn parse_cpu_instance_cases() {
    assert!(matches!(
        parse_cpu_instance("_Total"),
        Some(CpuInstance::Total)
    ));
    assert!(matches!(
        parse_cpu_instance("0,_Total"),
        Some(CpuInstance::GroupTotal)
    ));
    assert!(matches!(
        parse_cpu_instance("1,3"),
        Some(CpuInstance::Core { group: 1, index: 3 })
    ));
    assert!(parse_cpu_instance("x").is_none());
    assert!(parse_cpu_instance("0,").is_none());
}

#[test]
fn first_sample_after_open_is_none() {
    let mut sampler = PdhSampler::open().unwrap();

    assert!(sampler.sample().unwrap().is_none());
}

#[test]
fn pdh_english_paths_open_and_read() {
    let mut sampler = PdhSampler::open().unwrap();
    assert!(sampler.sample().unwrap().is_none());
    thread::sleep(Duration::from_millis(1_100));

    let reading = sampler
        .sample()
        .unwrap()
        .expect("second collection has rates");

    assert!(
        (0.0..=100.0).contains(&reading.cpu_pct),
        "cpu {}",
        reading.cpu_pct
    );
    assert!(reading.dpc_pct >= 0.0 && reading.isr_pct >= 0.0);
    assert!(reading.hard_faults_per_s >= 0.0);
    assert_eq!(reading.cores.len(), logical_cpus() as usize);
    assert!(!reading.disks.is_empty());
    for disk in &reading.disks {
        assert_ne!(disk.name, "_Total");
        assert!((0.0..=100.0).contains(&disk.active_pct), "{disk:?}");
        assert!(disk.latency_ms >= 0.0 && disk.queue_len >= 0.0);
    }
}

#[test]
fn reset_skips_next_sample() {
    let mut sampler = PdhSampler::open().unwrap();
    sampler.sample().unwrap();
    thread::sleep(Duration::from_millis(1_100));
    assert!(sampler.sample().unwrap().is_some());

    sampler.reset();

    assert!(sampler.sample().unwrap().is_none());
}

#[test]
fn memory_info_is_consistent() {
    let memory = memory_info().unwrap();

    assert!(memory.avail_bytes > 0);
    assert!(memory.commit_bytes > 0);
    assert!(memory.commit_bytes <= memory.commit_limit_bytes);
}

#[test]
fn snapshot_contains_this_process() {
    let mut snapshotter = ProcessSnapshotter::with_capacity(512 * 1024);
    let pid = std::process::id();

    let processes = snapshotter.snapshot().unwrap();

    assert!(processes.len() > 5);
    let me = processes
        .iter()
        .find(|p| p.pid == pid)
        .expect("own pid listed");
    assert!(me.name.to_lowercase().ends_with(".exe"), "name {}", me.name);
    assert!(me.threads >= 1);
    assert!(me.working_set > 0 && me.private_bytes > 0);
    assert!(me.context_switches > 0);
    assert!(me.create_time > 0);
    assert!(processes.iter().any(|p| p.pid == me.parent_pid));
}

#[test]
fn snapshot_counts_this_process_work() {
    let mut snapshotter = ProcessSnapshotter::with_capacity(512 * 1024);
    let pid = std::process::id();
    let before = own(&mut snapshotter, pid);

    let started = Instant::now();
    let mut spin = 0u64;
    while started.elapsed() < Duration::from_millis(200) {
        spin = std::hint::black_box(spin.wrapping_add(1));
    }
    let dir = tempfile_dir();
    std::fs::write(dir.join("work.bin"), vec![7u8; 1 << 20]).unwrap();
    let after = own(&mut snapshotter, pid);
    let _ = std::fs::remove_dir_all(dir);

    assert!(after.cpu_time_100ns >= before.cpu_time_100ns + 1_000_000);
    assert!(after.write_bytes >= before.write_bytes + (1 << 20));
}

#[test]
fn snapshot_grows_a_small_buffer() {
    let mut snapshotter = ProcessSnapshotter::with_capacity(16);

    assert!(!snapshotter.snapshot().unwrap().is_empty());
}

fn own(snapshotter: &mut ProcessSnapshotter, pid: u32) -> watchmypc_core::ProcessCounters {
    snapshotter
        .snapshot()
        .unwrap()
        .into_iter()
        .find(|p| p.pid == pid)
        .unwrap()
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("watchmypc-sources-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
