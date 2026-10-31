use std::thread;
use std::time::Duration;

use vestigium_collect::{CoreTimes, CpuSampler, core_loads, logical_cpus};

fn times(idle: u64, kernel: u64, user: u64, dpc: u64, interrupt: u64) -> CoreTimes {
    CoreTimes {
        idle,
        kernel,
        user,
        dpc,
        interrupt,
    }
}

#[test]
fn core_loads_are_shares_of_elapsed_time() {
    // Kernel time includes idle time, as Windows reports it.
    let prev = [times(0, 0, 0, 0, 0)];
    let curr = [times(600, 800, 200, 50, 10)];

    let loads = core_loads(&prev, &curr);

    assert_eq!(loads.len(), 1);
    assert!((loads[0].cpu_pct - 40.0).abs() < 1e-4);
    assert!((loads[0].dpc_pct - 5.0).abs() < 1e-4);
    assert!((loads[0].isr_pct - 1.0).abs() < 1e-4);
}

#[test]
fn core_loads_without_elapsed_time_are_zero() {
    let same = [times(10, 20, 5, 1, 1)];

    let loads = core_loads(&same, &same);

    assert_eq!(loads[0].cpu_pct, 0.0);
    assert_eq!(loads[0].dpc_pct, 0.0);
}

#[test]
fn core_loads_ignore_counters_that_go_back() {
    let prev = [times(900, 1_000, 100, 50, 10)];
    let curr = [times(100, 200, 50, 5, 1)];

    let loads = core_loads(&prev, &curr);

    assert_eq!(loads[0].cpu_pct, 0.0);
    assert!(loads[0].cpu_pct.is_finite());
}

#[test]
fn first_cpu_sample_is_none() {
    let mut sampler = CpuSampler::new();

    assert!(sampler.sample().unwrap().is_none());
}

#[test]
fn cpu_sampler_reads_every_core() {
    let mut sampler = CpuSampler::new();
    assert!(sampler.sample().unwrap().is_none());
    thread::sleep(Duration::from_millis(1_100));

    let reading = sampler.sample().unwrap().expect("second sample has rates");

    assert_eq!(reading.cores.len(), logical_cpus() as usize);
    for value in [reading.cpu_pct, reading.dpc_pct, reading.isr_pct] {
        assert!((0.0..=100.0).contains(&value), "{value}");
    }
    assert!(
        reading
            .cores
            .iter()
            .all(|c| (0.0..=100.0).contains(&c.cpu_pct))
    );
}

#[test]
fn cpu_reset_skips_next_sample() {
    let mut sampler = CpuSampler::new();
    sampler.sample().unwrap();
    thread::sleep(Duration::from_millis(200));
    assert!(sampler.sample().unwrap().is_some());

    sampler.reset();

    assert!(sampler.sample().unwrap().is_none());
}
