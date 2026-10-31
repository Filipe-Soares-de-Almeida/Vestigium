use vestigium_core::{ProcessCounters, top_processes};

fn counters(pid: u32, name: &str) -> ProcessCounters {
    ProcessCounters {
        pid,
        parent_pid: 4,
        name: name.to_string(),
        create_time: 1_000 + i64::from(pid),
        cpu_time_100ns: 0,
        working_set: 1 << 20,
        private_bytes: 1 << 20,
        read_bytes: 0,
        write_bytes: 0,
        threads: 1,
        context_switches: 0,
    }
}

#[test]
fn top_processes_ranks_cpu_memory_and_io() {
    let prev = vec![
        counters(10, "busy.exe"),
        counters(11, "big.exe"),
        counters(12, "disk.exe"),
    ];
    let mut curr = prev.clone();
    curr[0].cpu_time_100ns = 5_000_000;
    curr[1].private_bytes = 8 << 30;
    curr[2].write_bytes = 64 << 20;

    let top = top_processes(&prev, &curr, 1_000, 8, 1);

    let mut pids: Vec<u32> = top.iter().map(|s| s.pid).collect();
    pids.sort_unstable();
    assert_eq!(pids, vec![10, 11, 12]);
    assert_eq!(top[0].pid, 10, "sorted by CPU, highest first");
}

#[test]
fn cpu_pct_is_share_of_machine() {
    let prev = vec![counters(20, "work.exe")];
    let mut curr = prev.clone();
    curr[0].cpu_time_100ns = 10_000_000;

    let top = top_processes(&prev, &curr, 1_000, 8, 5);

    assert_eq!(top.len(), 1);
    assert!(
        (top[0].cpu_pct - 12.5).abs() < 1e-4,
        "got {}",
        top[0].cpu_pct
    );
}

#[test]
fn io_rates_are_per_second() {
    let prev = vec![counters(21, "copy.exe")];
    let mut curr = prev.clone();
    curr[0].read_bytes = 10_000;
    curr[0].write_bytes = 20_000;

    let top = top_processes(&prev, &curr, 5_000, 8, 5);

    assert!((top[0].read_bps - 2_000.0).abs() < 1e-3);
    assert!((top[0].write_bps - 4_000.0).abs() < 1e-3);
}

#[test]
fn pid_reuse_is_not_a_delta() {
    let mut old = counters(30, "old.exe");
    old.cpu_time_100ns = 900_000_000;
    old.write_bytes = 1 << 30;
    let mut new = counters(30, "new.exe");
    new.create_time = old.create_time + 1;
    new.cpu_time_100ns = 1_000;
    new.write_bytes = 10;

    let top = top_processes(&[old], &[new], 1_000, 8, 5);

    assert_eq!(top.len(), 1);
    assert_eq!(top[0].cpu_pct, 0.0);
    assert_eq!(top[0].write_bps, 0.0);
}

#[test]
fn exited_process_is_dropped() {
    let prev = vec![counters(7, "gone.exe"), counters(8, "stays.exe")];
    let curr = vec![counters(8, "stays.exe")];

    let top = top_processes(&prev, &curr, 1_000, 8, 5);

    assert!(top.iter().all(|s| s.pid != 7));
}

#[test]
fn idle_process_is_excluded() {
    let mut idle = counters(0, "Idle");
    let prev = vec![idle.clone()];
    idle.cpu_time_100ns = 70_000_000;
    idle.private_bytes = u64::MAX;

    let top = top_processes(&prev, &[idle], 1_000, 8, 5);

    assert!(top.is_empty());
}

#[test]
fn zero_elapsed_gives_zero_rates() {
    let prev = vec![counters(40, "fast.exe")];
    let mut curr = prev.clone();
    curr[0].cpu_time_100ns = 1_000;
    curr[0].read_bytes = 1_000;

    let top = top_processes(&prev, &curr, 0, 8, 5);

    assert_eq!(top[0].cpu_pct, 0.0);
    assert_eq!(top[0].read_bps, 0.0);
}
