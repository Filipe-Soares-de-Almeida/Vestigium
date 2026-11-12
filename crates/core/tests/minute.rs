use vestigium_core::{CoreLoad, DiskLoad, MinuteSample, Sample, minute_of, summarize_minute};

fn sample(ts_ms: i64, cpu_pct: f32, mem_avail_bytes: u64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct,
        dpc_pct: 0.5,
        isr_pct: 0.1,
        perf_limit_pct: Some(0.0),
        proc_perf_pct: Some(99.0),
        mem_total_bytes: 16 << 30,
        mem_avail_bytes,
        commit_bytes: 10 << 30,
        commit_limit_bytes: 32 << 30,
        hard_faults_per_s: Some(2.0),
        gpu_pct: None,
        probe_max_lag_ms: 0.5,
        sampler_lag_ms: 1.0,
        cores: vec![
            CoreLoad {
                cpu_pct,
                dpc_pct: 1.0,
                isr_pct: 0.5,
            };
            2
        ],
        disks: vec![DiskLoad {
            name: "0 C:".to_string(),
            active_pct: cpu_pct / 2.0,
            latency_ms: Some(1.0),
            queue_len: Some(0.0),
            bytes_per_s: Some(1_000.0),
        }],
    }
}

#[test]
fn minute_summary_takes_max_and_average() {
    let mut busy = sample(120_000, 60.0, 2 << 30);
    busy.cores[1].cpu_pct = 90.0;
    busy.cores[1].dpc_pct = 7.0;
    busy.gpu_pct = Some(40.0);
    busy.probe_max_lag_ms = 12.0;
    busy.disks[0].latency_ms = Some(25.0);
    busy.disks[0].bytes_per_s = Some(4_000.0);
    let samples = [
        sample(120_000, 10.0, 6 << 30),
        sample(121_000, 20.0, 5 << 30),
        busy,
    ];

    let minute: MinuteSample = summarize_minute(&samples).unwrap();

    assert_eq!(minute.ts_ms, 120_000);
    assert_eq!(minute.samples, 3);
    assert_eq!(minute.cpu_avg, 30.0);
    assert_eq!(minute.cpu_max, 60.0);
    assert_eq!(minute.core_max, 90.0);
    assert_eq!(minute.dpc_isr_max, 7.5);
    assert_eq!(minute.mem_total_bytes, 16 << 30);
    assert_eq!(minute.mem_avail_min, 2 << 30);
    assert_eq!(minute.gpu_max, Some(40.0));
    assert_eq!(minute.probe_max_lag_ms, 12.0);
    assert_eq!(minute.disks.len(), 1);
    assert_eq!(minute.disks[0].name, "0 C:");
    assert_eq!(minute.disks[0].active_max, 30.0);
    assert_eq!(minute.disks[0].latency_max_ms, 25.0);
    assert_eq!(minute.disks[0].bytes_avg, 2_000.0);
}

#[test]
fn minute_summary_of_nothing_is_none() {
    assert_eq!(summarize_minute(&[]), None);
}

#[test]
fn minute_starts_on_the_wall_clock_minute() {
    let minute = summarize_minute(&[sample(125_400, 5.0, 1 << 30)]).unwrap();

    assert_eq!(minute.ts_ms, 120_000);
    assert_eq!(minute.gpu_max, None);
}

#[test]
fn minute_of_handles_any_time() {
    assert_eq!(minute_of(-1), -60_000);
    assert_eq!(minute_of(59_999), 0);
    assert_eq!(minute_of(i64::MAX), i64::MAX - i64::MAX % 60_000);
    assert!(minute_of(i64::MIN) <= i64::MIN + 60_000);
}

#[test]
fn merged_minutes_weigh_by_sample_count() {
    let mut first = summarize_minute(
        &(0..3)
            .map(|s| sample(s * 1_000, 10.0, 4 << 30))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let second = summarize_minute(&[sample(50_000, 50.0, 1 << 30)]).unwrap();

    first.merge(&second);

    assert_eq!(first.samples, 4);
    assert_eq!(first.cpu_avg, 20.0);
    assert_eq!(first.cpu_max, 50.0);
    assert_eq!(first.mem_avail_min, 1 << 30);
    assert_eq!(first.disks[0].bytes_avg, 1_000.0);
}

#[test]
fn minute_skips_the_counters_a_sample_could_not_read() {
    let mut bare = sample(120_000, 20.0, 4 << 30);
    bare.perf_limit_pct = None;
    bare.hard_faults_per_s = None;
    bare.disks[0].latency_ms = None;
    bare.disks[0].bytes_per_s = None;
    let mut read = sample(121_000, 20.0, 4 << 30);
    read.perf_limit_pct = Some(80.0);
    read.hard_faults_per_s = Some(7.0);
    read.disks[0].latency_ms = Some(12.0);
    read.disks[0].bytes_per_s = Some(3_000.0);

    let minute = summarize_minute(&[bare, read]).unwrap();

    assert_eq!(minute.samples, 2);
    assert_eq!(minute.perf_limit_min, 80.0);
    assert_eq!(minute.hard_faults_max, 7.0);
    assert_eq!(minute.disks[0].latency_max_ms, 12.0);
    assert_eq!(minute.disks[0].bytes_avg, 3_000.0);
}

#[test]
fn a_minute_with_no_counter_readings_shows_no_latency_and_no_throttling() {
    let mut bare = sample(180_000, 20.0, 4 << 30);
    bare.perf_limit_pct = None;
    bare.disks[0].latency_ms = None;
    bare.disks[0].bytes_per_s = None;

    let minute = summarize_minute(&[bare]).unwrap();

    assert_eq!(minute.perf_limit_min, f32::MAX);
    assert_eq!(minute.disks[0].latency_max_ms, 0.0);
    assert_eq!(minute.disks[0].bytes_avg, 0.0);
}

#[test]
fn minute_keeps_the_lowest_performance_ceiling() {
    let mut held_back = sample(60_000, 20.0, 4 << 30);
    held_back.perf_limit_pct = Some(62.0);
    let mut free = sample(61_000, 20.0, 4 << 30);
    free.perf_limit_pct = Some(100.0);

    let minute = summarize_minute(&[free.clone(), held_back, free]).unwrap();

    assert_eq!(minute.perf_limit_min, 62.0);
}
