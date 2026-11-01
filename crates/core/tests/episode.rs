use vestigium_core::{
    CoreLoad, DiskLoad, Episode, EpisodeDetector, EpisodeKind, ProcessSample, ProcessTop, Sample,
};

const GIB: u64 = 1 << 30;

fn calm(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 10.0,
        dpc_pct: 0.5,
        isr_pct: 0.1,
        perf_limit_pct: 0.0,
        proc_perf_pct: 99.0,
        mem_total_bytes: 16 * GIB,
        mem_avail_bytes: 8 * GIB,
        commit_bytes: 10 * GIB,
        commit_limit_bytes: 32 * GIB,
        hard_faults_per_s: 2.0,
        gpu_pct: None,
        probe_max_lag_ms: 0.5,
        sampler_lag_ms: 1.0,
        cores: vec![
            CoreLoad {
                cpu_pct: 10.0,
                dpc_pct: 0.5,
                isr_pct: 0.1,
            };
            2
        ],
        disks: vec![DiskLoad {
            name: "0 C:".to_string(),
            active_pct: 5.0,
            latency_ms: 1.0,
            queue_len: 0.0,
            bytes_per_s: 1_000.0,
        }],
    }
}

fn memory_high(ts_ms: i64) -> Sample {
    Sample {
        mem_avail_bytes: GIB,
        ..calm(ts_ms)
    }
}

fn dpc_storm(ts_ms: i64) -> Sample {
    let mut sample = calm(ts_ms);
    sample.cores[1].dpc_pct = 12.0;
    sample
}

fn process(pid: u32, name: &str, private_bytes: u64) -> ProcessSample {
    ProcessSample {
        pid,
        name: name.to_string(),
        cpu_pct: 1.0,
        private_bytes,
        read_bps: 0.0,
        write_bps: 0.0,
        gpu_pct: 0.0,
    }
}

/// Feeds one sample per second from `from_s` to `to_s` inclusive, keeping what comes out.
fn feed(
    detector: &mut EpisodeDetector,
    from_s: i64,
    to_s: i64,
    make: fn(i64) -> Sample,
    sensitive: bool,
) -> Vec<Episode> {
    (from_s..=to_s)
        .flat_map(|s| detector.push_sample(&make(s * 1_000), sensitive))
        .collect()
}

#[test]
fn memory_episode_needs_two_minutes() {
    let mut short = EpisodeDetector::new();
    feed(&mut short, 0, 119, memory_high, false);
    assert!(feed(&mut short, 120, 140, calm, false).is_empty());

    let mut long = EpisodeDetector::new();
    feed(&mut long, 0, 121, memory_high, false);
    let episodes = feed(&mut long, 122, 140, calm, false);
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].kind, EpisodeKind::MemoryHigh);
    assert_eq!((episodes[0].start_ms, episodes[0].end_ms), (0, 121_000));
}

#[test]
fn episode_ends_after_ten_seconds_below() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 150, memory_high, false);
    assert!(feed(&mut detector, 151, 159, calm, false).is_empty());
    assert_eq!(feed(&mut detector, 160, 160, calm, false).len(), 1);
}

#[test]
fn short_dip_does_not_split_an_episode() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 100, memory_high, false);
    feed(&mut detector, 101, 105, calm, false);
    feed(&mut detector, 106, 200, memory_high, false);
    let episodes = feed(&mut detector, 201, 220, calm, false);
    assert_eq!(episodes.len(), 1);
    assert_eq!((episodes[0].start_ms, episodes[0].end_ms), (0, 200_000));
}

#[test]
fn culprit_is_the_largest_private_memory() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 60, memory_high, false);
    detector.push_top(&ProcessTop {
        ts_ms: 60_000,
        entries: vec![
            process(4, "browser.exe", 3 * GIB),
            process(5, "builder.exe", 5 * GIB),
        ],
    });
    feed(&mut detector, 61, 130, memory_high, false);
    detector.push_top(&ProcessTop {
        ts_ms: 130_000,
        entries: vec![process(4, "browser.exe", 6 * GIB)],
    });
    let episodes = feed(&mut detector, 131, 145, calm, false);
    let culprit = episodes[0].culprit.as_ref().expect("a culprit");
    assert_eq!((culprit.pid, culprit.name.as_str()), (4, "browser.exe"));
    assert_eq!(culprit.value, (6 * GIB) as f64);
}

#[test]
fn hints_only_outside_sensitive_mode() {
    let mut sensitive = EpisodeDetector::new();
    feed(&mut sensitive, 0, 10, dpc_storm, true);
    assert!(feed(&mut sensitive, 11, 30, calm, true).is_empty());

    let mut default = EpisodeDetector::new();
    feed(&mut default, 0, 10, dpc_storm, false);
    let episodes = feed(&mut default, 11, 30, calm, false);
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].kind, EpisodeKind::DpcIsrHint { core: 1 });
}

#[test]
fn hints_are_rate_limited() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 5, dpc_storm, false);
    let first = feed(&mut detector, 6, 179, calm, false);
    feed(&mut detector, 180, 185, dpc_storm, false);
    let second = feed(&mut detector, 186, 699, calm, false);
    feed(&mut detector, 700, 705, dpc_storm, false);
    let third = feed(&mut detector, 706, 730, calm, false);
    assert_eq!((first.len(), second.len(), third.len()), (1, 0, 1));
}

#[test]
fn close_all_marks_truncated() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 130, memory_high, false);
    let episodes = detector.close_all();
    assert_eq!(episodes.len(), 1);
    assert!(episodes[0].truncated);
    assert!(feed(&mut detector, 131, 150, calm, false).is_empty());
}

#[test]
fn pressure_is_open_once_the_episode_counts() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 100, memory_high, false);
    assert!(
        detector.open_pressure().is_empty(),
        "two minutes are not up yet"
    );

    feed(&mut detector, 101, 130, memory_high, false);
    assert_eq!(detector.open_pressure(), [EpisodeKind::MemoryHigh]);

    feed(&mut detector, 131, 135, calm, false);
    assert_eq!(
        detector.open_pressure(),
        [EpisodeKind::MemoryHigh],
        "a short dip keeps it open"
    );

    feed(&mut detector, 136, 145, calm, false);
    assert!(detector.open_pressure().is_empty());
}

#[test]
fn hints_are_not_pressure() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 10, dpc_storm, false);
    assert!(detector.open_pressure().is_empty());
}

#[test]
fn close_all_ends_the_pressure() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 130, memory_high, false);
    detector.close_all();
    assert!(detector.open_pressure().is_empty());
}
