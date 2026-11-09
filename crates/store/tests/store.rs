use std::time::Duration;

use vestigium_core::{
    CoreLoad, DiskLoad, MinuteSample, ProcessSample, ProcessSampleV1, ProcessTop, ProcessTopV1,
    Sample, SampleV1, summarize_minute,
};
use vestigium_store::{
    Batcher, SCHEMA_VERSION, Segment, Series, Store, StoreError, decode, encode,
};

fn sample(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 12.5,
        dpc_pct: 0.4,
        isr_pct: 0.1,
        perf_limit_pct: 0.0,
        proc_perf_pct: 98.0,
        mem_total_bytes: 32 << 30,
        mem_avail_bytes: 8 << 30,
        commit_bytes: 12 << 30,
        commit_limit_bytes: 40 << 30,
        hard_faults_per_s: 3.0,
        gpu_pct: Some(7.0),
        probe_max_lag_ms: 0.7,
        sampler_lag_ms: 1.2,
        cores: vec![
            CoreLoad {
                cpu_pct: 20.0,
                dpc_pct: 0.5,
                isr_pct: 0.1,
            };
            4
        ],
        disks: vec![DiskLoad {
            name: "0 C:".to_string(),
            active_pct: 3.0,
            latency_ms: 0.8,
            queue_len: 0.0,
            bytes_per_s: 120_000.0,
        }],
    }
}

fn top(ts_ms: i64) -> ProcessTop {
    ProcessTop {
        ts_ms,
        entries: vec![ProcessSample {
            pid: 4321,
            name: "example.exe".to_string(),
            cpu_pct: 5.0,
            private_bytes: 300 << 20,
            read_bps: 0.0,
            write_bps: 4096.0,
            gpu_pct: 1.5,
        }],
    }
}

fn sample_v1(ts_ms: i64) -> SampleV1 {
    let current = sample(ts_ms);
    SampleV1 {
        ts_ms,
        cpu_pct: current.cpu_pct,
        dpc_pct: current.dpc_pct,
        isr_pct: current.isr_pct,
        perf_limit_pct: current.perf_limit_pct,
        proc_perf_pct: current.proc_perf_pct,
        mem_avail_bytes: current.mem_avail_bytes,
        commit_bytes: current.commit_bytes,
        commit_limit_bytes: current.commit_limit_bytes,
        hard_faults_per_s: current.hard_faults_per_s,
        probe_max_lag_ms: current.probe_max_lag_ms,
        sampler_lag_ms: current.sampler_lag_ms,
        cores: current.cores,
        disks: current.disks,
    }
}

fn top_v1(ts_ms: i64) -> ProcessTopV1 {
    ProcessTopV1 {
        ts_ms,
        entries: vec![ProcessSampleV1 {
            pid: 4321,
            name: "example.exe".to_string(),
            cpu_pct: 5.0,
            private_bytes: 300 << 20,
            read_bps: 0.0,
            write_bps: 4096.0,
        }],
    }
}

fn samples_segment(samples: &[Sample]) -> Segment {
    Segment {
        series: Series::Samples,
        start_ms: samples.first().map_or(0, |s| s.ts_ms),
        end_ms: samples.last().map_or(0, |s| s.ts_ms),
        data: encode(samples),
    }
}

fn minutes_in(segments: &[Segment]) -> Vec<MinuteSample> {
    segments
        .iter()
        .filter(|s| s.series == Series::Minutes)
        .flat_map(|s| decode::<MinuteSample>(&s.data).unwrap())
        .collect()
}

fn segment(start_ms: i64, end_ms: i64) -> Segment {
    Segment {
        series: Series::Samples,
        start_ms,
        end_ms,
        data: encode(&[sample(start_ms)]),
    }
}

#[test]
fn open_sets_wal_and_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("vestigium.db");

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.journal_mode().unwrap(), "wal");
}

#[test]
fn open_keeps_the_page_cache_small() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("vestigium.db")).unwrap();

    assert_eq!(store.cache_size_kib().unwrap(), 256);
}

#[test]
fn reopen_keeps_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.pragma_update(None, "user_version", 99).unwrap();
    }

    let err = Store::open(&path).expect_err("newer schema must be refused");

    assert!(matches!(
        err,
        StoreError::NewerSchema {
            found: 99,
            supported: SCHEMA_VERSION
        }
    ));
    let raw = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = raw
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 99);
    let tables: i64 = raw
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))
        .unwrap();
    assert_eq!(tables, 0);
}

#[test]
fn segments_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    let samples = vec![sample(1_000), sample(2_000), sample(3_000)];

    store
        .append(&[Segment {
            series: Series::Samples,
            start_ms: 1_000,
            end_ms: 3_000,
            data: encode(&samples),
        }])
        .unwrap();
    let read = store.segments(Series::Samples, 0, 10_000).unwrap();

    assert_eq!(read.len(), 1);
    assert_eq!(decode::<Sample>(&read[0].data).unwrap(), samples);
}

#[test]
fn segments_query_returns_overlaps() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    store
        .append(&[segment(0, 10), segment(10, 20), segment(20, 30)])
        .unwrap();

    let found = store.segments(Series::Samples, 15, 25).unwrap();

    let spans: Vec<(i64, i64)> = found.iter().map(|s| (s.start_ms, s.end_ms)).collect();
    assert_eq!(spans, vec![(10, 20), (20, 30)]);
}

#[test]
fn segments_query_filters_by_series() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    store
        .append(&[
            segment(0, 10),
            Segment {
                series: Series::Processes,
                start_ms: 0,
                end_ms: 10,
                data: encode(&[top(0)]),
            },
        ])
        .unwrap();

    let found = store.segments(Series::Processes, 0, 10).unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].series, Series::Processes);
}

#[test]
fn encode_writes_compressed_format() {
    let samples: Vec<Sample> = (0..60).map(|s| sample(s * 1_000)).collect();
    let data = encode(&samples);
    let raw = postcard::to_stdvec(&samples).unwrap();

    assert_eq!(data[0], 2);
    assert!(
        data.len() < raw.len() / 2,
        "{} vs {}",
        data.len(),
        raw.len()
    );
    assert_eq!(decode::<Sample>(&data).unwrap(), samples);
}

#[test]
fn uncompressed_format_still_decodes() {
    let samples = vec![sample(1_000), sample(2_000)];
    let mut data = vec![1u8];
    data.extend(postcard::to_stdvec(&samples).unwrap());

    assert_eq!(decode::<Sample>(&data).unwrap(), samples);
}

#[test]
fn corrupt_compressed_segment_is_an_error() {
    let data = [2u8, 0xFF, 0xFF, 0xFF, 0x00, 0x13];

    assert!(matches!(decode::<Sample>(&data), Err(StoreError::Corrupt)));
}

#[test]
fn decode_rejects_unknown_format() {
    let mut data = encode(&[sample(0)]);
    data[0] = 3;

    assert!(matches!(
        decode::<Sample>(&data),
        Err(StoreError::UnknownFormat(3))
    ));
    assert!(matches!(
        decode::<Sample>(&[]),
        Err(StoreError::EmptySegment)
    ));
}

#[test]
fn batcher_is_due_after_interval() {
    let mut batcher = Batcher::new(Duration::from_secs(5));
    assert!(!batcher.due(1_000_000));

    batcher.push_sample(sample(1_000_000));

    assert!(!batcher.due(1_004_999));
    assert!(batcher.due(1_005_000));
}

#[test]
fn batcher_take_empties_the_buffer() {
    let mut batcher = Batcher::new(Duration::from_secs(5));
    batcher.push_sample(sample(1_000));
    batcher.push_sample(sample(2_000));
    batcher.push_processes(top(2_000));

    let segments = batcher.take();

    assert_eq!(segments.len(), 2);
    let samples = segments
        .iter()
        .find(|s| s.series == Series::Samples)
        .unwrap();
    assert_eq!((samples.start_ms, samples.end_ms), (1_000, 2_000));
    assert_eq!(decode::<Sample>(&samples.data).unwrap().len(), 2);
    assert!(batcher.take().is_empty());
}

#[test]
fn batcher_rolls_up_each_wall_clock_minute() {
    let mut batcher = Batcher::new(Duration::from_secs(3600));
    for ts_ms in [58_000, 59_000, 60_000, 61_000, 119_000, 120_000, 121_000] {
        batcher.push_sample(sample(ts_ms));
    }

    let minutes = minutes_in(&batcher.take());

    let spans: Vec<(i64, u16)> = minutes.iter().map(|m| (m.ts_ms, m.samples)).collect();
    assert_eq!(spans, vec![(0, 2), (60_000, 3)]);
}

#[test]
fn take_final_includes_the_open_minute() {
    let mut batcher = Batcher::new(Duration::from_secs(3600));
    batcher.push_sample(sample(60_000));
    batcher.push_sample(sample(61_000));
    assert!(minutes_in(&batcher.take()).is_empty());

    let minutes = minutes_in(&batcher.take_final());

    assert_eq!(minutes.len(), 1);
    assert_eq!((minutes[0].ts_ms, minutes[0].samples), (60_000, 2));
    assert!(batcher.take_final().is_empty());
}

#[test]
fn v1_segments_read_as_current_types() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    store
        .append(&[
            Segment {
                series: Series::SamplesV1,
                start_ms: 1_000,
                end_ms: 2_000,
                data: encode(&[sample_v1(1_000), sample_v1(2_000)]),
            },
            Segment {
                series: Series::ProcessesV1,
                start_ms: 2_000,
                end_ms: 2_000,
                data: encode(&[top_v1(2_000)]),
            },
        ])
        .unwrap();

    let samples = store.samples(0, 10_000).unwrap();
    let tops = store.tops(0, 10_000).unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].cpu_pct, 12.5);
    assert_eq!(samples[0].mem_total_bytes, 0);
    assert_eq!(samples[0].gpu_pct, None);
    assert_eq!(tops.len(), 1);
    assert_eq!(tops[0].entries[0].name, "example.exe");
    assert_eq!(tops[0].entries[0].gpu_pct, 0.0);
}

#[test]
fn samples_merge_versions_in_time_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    store
        .append(&[samples_segment(&[sample(3_000), sample(4_000)])])
        .unwrap();
    store
        .append(&[Segment {
            series: Series::SamplesV1,
            start_ms: 1_000,
            end_ms: 2_000,
            data: encode(&[sample_v1(1_000), sample_v1(2_000)]),
        }])
        .unwrap();

    let samples = store.samples(i64::MIN, i64::MAX).unwrap();

    let times: Vec<i64> = samples.iter().map(|s| s.ts_ms).collect();
    assert_eq!(times, vec![1_000, 2_000, 3_000, 4_000]);
    assert_eq!(samples[3].gpu_pct, Some(7.0));
}

#[test]
fn reads_trim_to_the_requested_range() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    let minute: Vec<Sample> = (0..60).map(|s| sample(s * 1_000)).collect();
    store.append(&[samples_segment(&minute)]).unwrap();

    let samples = store.samples(30_000, 40_000).unwrap();

    assert_eq!(samples.len(), 11);
    assert_eq!(samples.first().map(|s| s.ts_ms), Some(30_000));
    assert_eq!(samples.last().map(|s| s.ts_ms), Some(40_000));
}

#[test]
fn duplicate_minutes_are_merged() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
    let before_stop: Vec<Sample> = (0..20).map(|s| sample(60_000 + s * 1_000)).collect();
    let after_start: Vec<Sample> = (30..60).map(|s| sample(60_000 + s * 1_000)).collect();
    for part in [&before_stop, &after_start] {
        let minute = summarize_minute(part).unwrap();
        store
            .append(&[Segment {
                series: Series::Minutes,
                start_ms: minute.ts_ms,
                end_ms: minute.ts_ms,
                data: encode(&[minute]),
            }])
            .unwrap();
    }

    let minutes = store.minutes(0, 600_000).unwrap();

    assert_eq!(minutes.len(), 1);
    assert_eq!((minutes[0].ts_ms, minutes[0].samples), (60_000, 50));
}
