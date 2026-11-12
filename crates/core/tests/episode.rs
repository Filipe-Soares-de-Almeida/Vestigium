use vestigium_core::{
    CoreLoad, DiskLoad, Episode, EpisodeDetector, EpisodeKind, PressureSpan, ProcessSample,
    ProcessTop, Sample,
};

const GIB: u64 = 1 << 30;

fn calm(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 10.0,
        dpc_pct: 0.5,
        isr_pct: 0.1,
        perf_limit_pct: Some(0.0),
        proc_perf_pct: Some(99.0),
        mem_total_bytes: 16 * GIB,
        mem_avail_bytes: 8 * GIB,
        commit_bytes: 10 * GIB,
        commit_limit_bytes: 32 * GIB,
        hard_faults_per_s: Some(2.0),
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
            latency_ms: Some(1.0),
            queue_len: Some(0.0),
            bytes_per_s: Some(1_000.0),
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
    sample.cores[1].dpc_pct = 20.0;
    sample.cores[1].isr_pct = 5.0;
    sample
}

/// 86% of memory in use.
fn memory_trimmed(ts_ms: i64) -> Sample {
    Sample {
        mem_avail_bytes: 16 * GIB * 14 / 100,
        ..calm(ts_ms)
    }
}

/// 22% DPC plus interrupt time on core 0.
fn dpc_busy_core0(ts_ms: i64) -> Sample {
    let mut sample = calm(ts_ms);
    sample.cores[0].dpc_pct = 14.0;
    sample.cores[0].isr_pct = 8.0;
    sample
}

fn cpu_saturated(ts_ms: i64) -> Sample {
    let mut sample = calm(ts_ms);
    sample.cpu_pct = 100.0;
    sample.cores.iter_mut().for_each(|c| c.cpu_pct = 100.0);
    sample
}

fn busy(pid: u32, name: &str, cpu_pct: f32) -> ProcessSample {
    ProcessSample {
        cpu_pct,
        ..process(pid, name, GIB)
    }
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
fn memory_held_above_85_percent_is_high() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 150, memory_trimmed, false);
    let episodes = feed(&mut detector, 151, 170, calm, false);
    assert_eq!(episodes.len(), 1);
    assert_eq!(episodes[0].kind, EpisodeKind::MemoryHigh);
}

#[test]
fn ordinary_dpc_load_is_no_hint() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 60, dpc_busy_core0, false);
    assert!(feed(&mut detector, 61, 80, calm, false).is_empty());
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
fn cpu_culprit_sums_every_process_of_a_program() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 30, cpu_saturated, false);
    detector.push_top(&ProcessTop {
        ts_ms: 30_000,
        entries: vec![
            busy(7, "watcher.exe", 12.0),
            busy(20, "rustc.exe", 9.0),
            busy(21, "rustc.exe", 8.0),
        ],
    });
    feed(&mut detector, 31, 61, cpu_saturated, false);
    detector.push_top(&ProcessTop {
        ts_ms: 61_000,
        entries: vec![busy(7, "watcher.exe", 12.0), busy(22, "rustc.exe", 10.0)],
    });
    feed(&mut detector, 62, 70, cpu_saturated, false);
    let episodes = feed(&mut detector, 71, 90, calm, false);

    assert_eq!(episodes[0].kind, EpisodeKind::CpuSaturated);
    let culprit = episodes[0].culprit.as_ref().expect("a culprit");
    assert_eq!((culprit.pid, culprit.name.as_str()), (22, "rustc.exe"));
    assert!((culprit.value - 13.5).abs() < 1e-4, "got {}", culprit.value);
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
fn a_disk_without_a_latency_reading_starts_no_hint() {
    let unread = |ts_ms: i64| {
        let mut sample = calm(ts_ms);
        sample.disks[0].active_pct = 100.0;
        sample.disks[0].latency_ms = None;
        sample
    };
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 30, unread, false);
    let ended = feed(&mut detector, 31, 40, calm, false);

    assert!(ended.iter().all(|e| !e.kind.is_hint()));
}

#[test]
fn close_all_ends_the_pressure() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 130, memory_high, false);
    detector.close_all();
    assert!(detector.open_pressure().is_empty());
}

#[test]
fn open_pressure_follows_the_timeline_rule() {
    let everything = |ts_ms: i64| {
        let mut sample = memory_high(ts_ms);
        for core in &mut sample.cores {
            core.cpu_pct = 99.0;
            core.dpc_pct = 12.0;
        }
        sample.disks[0].active_pct = 100.0;
        sample.disks[0].latency_ms = Some(150.0);
        sample
    };
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 130, everything, false);
    let open = detector.open_pressure();

    let ended = feed(&mut detector, 131, 145, calm, false);
    assert!(
        ended.iter().any(|e| e.kind.is_hint()),
        "the hints were found too"
    );
    let stored: Vec<EpisodeKind> = ended
        .into_iter()
        .map(|e| e.kind)
        .filter(|kind| !kind.is_hint())
        .collect();
    assert_eq!(open.len(), 3);
    assert_eq!(stored.len(), open.len());
    assert!(stored.iter().all(|kind| open.contains(kind)));
}

#[test]
fn open_spans_start_where_the_episode_starts() {
    let mut detector = EpisodeDetector::new();
    feed(&mut detector, 0, 9, calm, false);
    feed(&mut detector, 10, 140, memory_high, false);
    feed(&mut detector, 141, 141, dpc_storm, false);

    assert_eq!(
        detector.open_spans(),
        [PressureSpan {
            kind: EpisodeKind::MemoryHigh,
            start_ms: 10_000,
            end_ms: None,
        }]
    );
}

#[test]
fn a_span_covers_its_ends_and_runs_on_while_open() {
    let ended = PressureSpan {
        kind: EpisodeKind::CpuSaturated,
        start_ms: 2_000,
        end_ms: Some(5_000),
    };
    let covered: Vec<i64> = (1..=6)
        .map(|s| s * 1_000)
        .filter(|&ts| ended.covers(ts))
        .collect();
    assert_eq!(covered, [2_000, 3_000, 4_000, 5_000]);

    let open = PressureSpan {
        end_ms: None,
        ..ended
    };
    assert!(!open.covers(1_000));
    assert!(open.covers(i64::MAX));
}
