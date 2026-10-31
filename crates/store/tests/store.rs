use std::time::Duration;

use watchmypc_core::{CoreLoad, DiskLoad, ProcessSample, ProcessTop, Sample};
use watchmypc_store::{
    Batcher, Ring, SCHEMA_VERSION, Segment, Series, Store, StoreError, decode, encode,
};

fn sample(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 12.5,
        dpc_pct: 0.4,
        isr_pct: 0.1,
        perf_limit_pct: 0.0,
        proc_perf_pct: 98.0,
        mem_avail_bytes: 8 << 30,
        commit_bytes: 12 << 30,
        commit_limit_bytes: 40 << 30,
        hard_faults_per_s: 3.0,
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
        }],
    }
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
    let path = dir.path().join("nested").join("watchmypc.db");

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.journal_mode().unwrap(), "wal");
}

#[test]
fn reopen_keeps_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("watchmypc.db");
    drop(Store::open(&path).unwrap());

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("watchmypc.db");
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
    let mut store = Store::open(&dir.path().join("watchmypc.db")).unwrap();
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
    let mut store = Store::open(&dir.path().join("watchmypc.db")).unwrap();
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
    let mut store = Store::open(&dir.path().join("watchmypc.db")).unwrap();
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
fn decode_rejects_unknown_format() {
    let mut data = encode(&[sample(0)]);
    data[0] = 2;

    assert!(matches!(
        decode::<Sample>(&data),
        Err(StoreError::UnknownFormat(2))
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
fn ring_drops_the_oldest() {
    let mut ring = Ring::with_capacity(3);
    for n in 1..=5 {
        ring.push(n);
    }

    assert_eq!(ring.len(), 3);
    assert_eq!(ring.iter().copied().collect::<Vec<_>>(), vec![3, 4, 5]);
}
