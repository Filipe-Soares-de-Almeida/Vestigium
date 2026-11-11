use vestigium_core::{
    BUSY_ABOVE, DiskLoad, DiskMinute, EpisodeKind, MinuteSample, PcState, Reading, Sample, pc_state,
};

fn sample(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 20.0,
        dpc_pct: 0.5,
        isr_pct: 0.1,
        perf_limit_pct: Some(100.0),
        proc_perf_pct: Some(99.0),
        mem_total_bytes: 16 << 30,
        mem_avail_bytes: 12 << 30,
        commit_bytes: 6 << 30,
        commit_limit_bytes: 32 << 30,
        hard_faults_per_s: Some(0.0),
        gpu_pct: Some(10.0),
        probe_max_lag_ms: 1.5,
        sampler_lag_ms: 1.0,
        cores: Vec::new(),
        disks: vec![disk("0 C:", 15.0, 2.0), disk("1 D:", 70.0, 30.0)],
    }
}

fn disk(name: &str, active_pct: f32, latency_ms: f32) -> DiskLoad {
    DiskLoad {
        name: name.to_string(),
        active_pct,
        latency_ms: Some(latency_ms),
        queue_len: Some(0.0),
        bytes_per_s: Some(0.0),
    }
}

fn reading(cpu_pct: f32, mem_used_pct: f32, disk_active_pct: f32, gpu_pct: f32) -> Reading {
    Reading {
        ts_ms: 0,
        cpu_pct,
        mem_used_pct: Some(mem_used_pct),
        disk_active_pct,
        disk_latency_ms: 1.0,
        gpu_pct: Some(gpu_pct),
        probe_lag_ms: 1.0,
    }
}

#[test]
fn reading_of_a_sample_ignores_a_latency_that_was_not_read() {
    let mut bare = sample(5_000);
    bare.disks.iter_mut().for_each(|d| d.latency_ms = None);

    let reading = Reading::of_sample(&bare);

    assert_eq!(reading.disk_active_pct, 70.0);
    assert_eq!(reading.disk_latency_ms, 0.0);
}

#[test]
fn reading_of_a_sample_takes_the_busiest_disk() {
    let reading = Reading::of_sample(&sample(5_000));

    assert_eq!(reading.ts_ms, 5_000);
    assert_eq!(reading.cpu_pct, 20.0);
    assert_eq!(reading.mem_used_pct, Some(25.0));
    assert_eq!(reading.disk_active_pct, 70.0);
    assert_eq!(reading.disk_latency_ms, 30.0);
    assert_eq!(reading.gpu_pct, Some(10.0));
    assert_eq!(reading.probe_lag_ms, 1.5);
}

#[test]
fn memory_is_unknown_without_a_total() {
    let mut old = sample(0);
    old.mem_total_bytes = 0;
    old.disks.clear();

    let reading = Reading::of_sample(&old);

    assert_eq!(reading.mem_used_pct, None);
    assert_eq!(reading.disk_active_pct, 0.0);
}

fn two_disk_minute() -> MinuteSample {
    MinuteSample {
        ts_ms: 120_000,
        samples: 60,
        cpu_avg: 12.0,
        cpu_max: 64.0,
        core_max: 100.0,
        dpc_isr_max: 3.0,
        mem_total_bytes: 8 << 30,
        mem_avail_min: 2 << 30,
        commit_max: 0,
        hard_faults_max: 0.0,
        gpu_max: None,
        perf_limit_min: 100.0,
        probe_max_lag_ms: 900.0,
        disks: vec![
            DiskMinute {
                name: "0 C:".to_string(),
                active_max: 40.0,
                latency_max_ms: 80.0,
                bytes_avg: 0.0,
            },
            DiskMinute {
                name: "1 D:".to_string(),
                active_max: 55.0,
                latency_max_ms: 4.0,
                bytes_avg: 0.0,
            },
        ],
    }
}

#[test]
fn reading_of_a_minute_takes_its_peaks() {
    let reading = Reading::of_minute(&two_disk_minute());

    assert_eq!(reading.ts_ms, 120_000);
    assert_eq!(reading.cpu_pct, 64.0);
    assert_eq!(reading.mem_used_pct, Some(75.0));
    assert_eq!(reading.disk_active_pct, 55.0);
    assert_eq!(reading.disk_latency_ms, 80.0);
    assert_eq!(reading.gpu_pct, None);
    assert_eq!(reading.probe_lag_ms, 900.0);
}

#[test]
fn reading_on_a_disk_reads_that_disk_alone() {
    let reading = Reading::of_sample_on(&sample(5_000), "0 C:");

    assert_eq!(reading.disk_active_pct, 15.0);
    assert_eq!(reading.disk_latency_ms, 2.0);
    assert_eq!(reading.cpu_pct, 20.0, "the other measures stay");
    assert_eq!(reading.mem_used_pct, Some(25.0));
}

#[test]
fn reading_on_a_disk_of_a_minute_reads_that_disk_alone() {
    let reading = Reading::of_minute_on(&two_disk_minute(), "1 D:");

    assert_eq!(reading.disk_active_pct, 55.0);
    assert_eq!(reading.disk_latency_ms, 4.0);
    assert_eq!(reading.cpu_pct, 64.0);
}

#[test]
fn reading_on_a_missing_disk_reads_no_disk() {
    let reading = Reading::of_sample_on(&sample(0), "2 E:");

    assert_eq!(reading.disk_active_pct, 0.0);
    assert_eq!(reading.disk_latency_ms, 0.0);
}

#[test]
fn peak_keeps_the_higher_value_of_each() {
    let mut first = reading(10.0, 50.0, 90.0, 5.0);
    first.ts_ms = 1_000;
    first.gpu_pct = None;
    let mut second = reading(80.0, 40.0, 20.0, 30.0);
    second.ts_ms = 2_000;
    second.probe_lag_ms = 7_000.0;

    first.peak(&second);

    assert_eq!(first.ts_ms, 1_000);
    assert_eq!(first.cpu_pct, 80.0);
    assert_eq!(first.mem_used_pct, Some(50.0));
    assert_eq!(first.disk_active_pct, 90.0);
    assert_eq!(first.gpu_pct, Some(30.0));
    assert_eq!(first.probe_lag_ms, 7_000.0);
}

#[test]
fn minute_state_follows_the_load_rule() {
    let state = |r: Reading| pc_state(Some(&r), false);

    assert_eq!(state(reading(BUSY_ABOVE, 30.0, 5.0, 0.0)), PcState::Calm);
    assert_eq!(state(reading(39.0, 30.0, 5.0, 0.0)), PcState::Busy);
    assert_eq!(state(reading(5.0, 30.0, 50.0, 0.0)), PcState::Busy);
    assert_eq!(state(reading(5.0, 30.0, 5.0, 60.0)), PcState::Calm);
    assert_eq!(state(reading(5.0, 30.0, 5.0, 100.0)), PcState::Busy);
    assert_eq!(state(reading(5.0, 73.0, 5.0, 0.0)), PcState::Calm);
    assert_eq!(state(reading(5.0, 74.0, 5.0, 0.0)), PcState::Busy);
}

#[test]
fn full_load_without_an_episode_is_busy() {
    let state = |r: Reading| pc_state(Some(&r), false);

    assert_eq!(state(reading(100.0, 30.0, 5.0, 0.0)), PcState::Busy);
    assert_eq!(state(reading(5.0, 30.0, 100.0, 0.0)), PcState::Busy);
    assert_eq!(state(reading(5.0, 95.0, 5.0, 100.0)), PcState::Busy);
}

#[test]
fn episode_minutes_are_under_pressure() {
    let calm = reading(5.0, 30.0, 5.0, 0.0);

    assert_eq!(pc_state(Some(&calm), true), PcState::Pressure);
}

#[test]
fn a_span_without_readings_is_suspended() {
    assert_eq!(pc_state(None, false), PcState::Suspended);
    assert_eq!(pc_state(None, true), PcState::Suspended);
}

#[test]
fn hints_are_not_pressure() {
    assert!(EpisodeKind::DpcIsrHint { core: 0 }.is_hint());
    assert!(
        EpisodeKind::DiskLatencyHint {
            disk: "0 C:".to_string()
        }
        .is_hint()
    );
    assert!(!EpisodeKind::MemoryHigh.is_hint());
    assert!(!EpisodeKind::CpuSaturated.is_hint());
}
