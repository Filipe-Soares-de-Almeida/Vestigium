use std::collections::BTreeSet;
use std::path::Path;

use vestigium_core::{
    Cause, Channel, ColorTheme, Confidence, CoreLoad, DetailLevel, Episode, EpisodeKind,
    EventMeaning, Gap, GapReason, LanguagePref, MachineInfo, Occurrence, OccurrenceKind, Retention,
    Sample, Settings, Severity, Suspect, Trigger, WinEvent, Window,
};
use vestigium_store::{Batch, SCHEMA_VERSION, Segment, Series, Store, StoreError, encode};

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;

fn sample(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 10.0,
        dpc_pct: 0.2,
        isr_pct: 0.1,
        perf_limit_pct: 0.0,
        proc_perf_pct: 99.0,
        mem_total_bytes: 16 << 30,
        mem_avail_bytes: 8 << 30,
        commit_bytes: 8 << 30,
        commit_limit_bytes: 24 << 30,
        hard_faults_per_s: 1.0,
        gpu_pct: None,
        probe_max_lag_ms: 0.4,
        sampler_lag_ms: 0.8,
        cores: vec![
            CoreLoad {
                cpu_pct: 10.0,
                dpc_pct: 0.2,
                isr_pct: 0.1,
            };
            2
        ],
        disks: Vec::new(),
    }
}

fn segment(series: Series, start_ms: i64, end_ms: i64) -> Segment {
    Segment {
        series,
        start_ms,
        end_ms,
        data: encode(&[sample(start_ms)]),
    }
}

fn machine() -> MachineInfo {
    MachineInfo {
        logical_cpus: 2,
        mem_total_bytes: 16 << 30,
        disks: Vec::new(),
    }
}

fn occurrence(start_ms: i64, end_ms: i64) -> Occurrence {
    Occurrence {
        kind: OccurrenceKind::Freeze,
        start_ms,
        end_ms,
        severity: Severity::Medium,
        triggers: vec![Trigger::Freeze {
            start_ms,
            end_ms,
            lag_ms: (end_ms - start_ms) as f32,
        }],
        window: Window {
            from_ms: start_ms - 60_000,
            to_ms: end_ms + 30_000,
            samples: vec![sample(start_ms - 1_000), sample(end_ms + 1_000)],
            tops: Vec::new(),
            events: Vec::new(),
            hangs: Vec::new(),
            gaps: Vec::new(),
            machine: machine(),
        },
        suspects: vec![Suspect {
            cause: Cause::Memory {
                process: Some("builder.exe".to_string()),
            },
            confidence: Confidence::Medium,
            evidence: Vec::new(),
            next_steps: Vec::new(),
        }],
        facts: Vec::new(),
        sensitive: false,
        truncated: false,
    }
}

fn episode(start_ms: i64, end_ms: i64) -> Episode {
    Episode {
        kind: EpisodeKind::CpuSaturated,
        start_ms,
        end_ms,
        peak: 99.0,
        culprit: None,
        truncated: false,
    }
}

fn event(ts_ms: i64, record_id: u64) -> WinEvent {
    WinEvent {
        ts_ms,
        channel: Channel::System,
        provider: "stornvme".to_string(),
        id: 129,
        record_id,
        level: 3,
        meaning: EventMeaning::StorageReset,
        data_xml: "<EventData/>".to_string(),
    }
}

const MEMORY_DIAGNOSTICS: &str = "Microsoft-Windows-MemoryDiagnostics-Results";

fn memory_diagnostic(ts_ms: i64, record_id: u64) -> WinEvent {
    WinEvent {
        provider: MEMORY_DIAGNOSTICS.to_string(),
        id: 1201,
        level: 4,
        meaning: EventMeaning::MemoryDiagnostic,
        ..event(ts_ms, record_id)
    }
}

fn gap(start_ms: i64, end_ms: i64) -> Gap {
    Gap {
        start_ms,
        end_ms,
        reason: GapReason::Suspended,
    }
}

fn open(dir: &Path) -> Store {
    Store::open(&dir.join("vestigium.db")).unwrap()
}

#[test]
fn migration_from_v1_keeps_segments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "CREATE TABLE segment (
                id INTEGER PRIMARY KEY, series INTEGER NOT NULL, start_ms INTEGER NOT NULL,
                end_ms INTEGER NOT NULL, data BLOB NOT NULL);
             CREATE INDEX segment_start ON segment (start_ms);
             PRAGMA user_version = 1;",
        )
        .unwrap();
        raw.execute(
            "INSERT INTO segment (series, start_ms, end_ms, data) VALUES (3, 1000, 2000, ?1)",
            [encode(&[sample(1_000)])],
        )
        .unwrap();
    }

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(store.samples(0, 10_000).unwrap(), vec![sample(1_000)]);
    assert!(store.occurrences(i64::MIN, i64::MAX).unwrap().is_empty());
}

#[test]
fn migration_repairs_memory_diagnostics_stored_without_a_meaning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "DROP INDEX tweak_change_key;
             PRAGMA user_version = 9;
             INSERT INTO event (ts_ms, channel, provider, event_id, record_id, level, meaning, data_xml)
             VALUES (1000, 1, 'stornvme', 129, 1, 3, 1, '<EventData/>'),
                    (2000, 1, 'Microsoft-Windows-MemoryDiagnostics-Results', 1201, 2, 4, 0,
                     '<EventData/>');",
        )
        .unwrap();
    }

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(
        store.events(i64::MIN, i64::MAX).unwrap(),
        vec![event(1_000, 1), memory_diagnostic(2_000, 2)]
    );
}

#[test]
fn a_storm_occurrence_is_stored_and_opened() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let mut stored = occurrence(10_000, 12_500);
    stored.triggers = (0..40_000)
        .map(|i| Trigger::Reported {
            event: WinEvent {
                ts_ms: 10_000 + i,
                channel: Channel::System,
                provider: "disk".to_string(),
                id: 153,
                record_id: i as u64,
                level: 2,
                meaning: EventMeaning::DiskRetry,
                data_xml: "x".repeat(500),
            },
        })
        .collect();

    let id = store.insert_occurrence(&stored).unwrap();

    assert_eq!(store.occurrence(id).unwrap(), Some(stored));
}

#[test]
fn occurrence_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let stored = occurrence(10_000, 12_500);

    let id = store.insert_occurrence(&stored).unwrap();

    assert_eq!(store.occurrence(id).unwrap(), Some(stored));
    let listed = store.occurrences(0, 20_000).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].kind, OccurrenceKind::Freeze);
    assert_eq!(listed[0].severity, Severity::Medium);
    assert_eq!(
        listed[0].cause,
        Cause::Memory {
            process: Some("builder.exe".to_string())
        }
    );
    assert_eq!(store.occurrence(id + 1).unwrap(), None);

    let episode_id = store.insert_episode(&episode(5_000, 70_000)).unwrap();
    assert!(episode_id > 0);
    assert_eq!(
        store.episodes(0, 20_000).unwrap(),
        vec![episode(5_000, 70_000)]
    );
}

#[test]
fn events_are_unique_per_channel_and_record() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let batch = Batch {
        events: vec![event(1_000, 42), event(1_000, 42)],
        ..Batch::default()
    };

    store.append_batch(&batch).unwrap();
    store.append_batch(&batch).unwrap();

    assert_eq!(store.events(0, 10_000).unwrap(), vec![event(1_000, 42)]);
}

#[test]
fn every_event_meaning_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let stored: Vec<WinEvent> = EventMeaning::ALL
        .into_iter()
        .zip(1..)
        .map(|(meaning, n)| WinEvent {
            meaning,
            ..event(n * 1_000, n as u64)
        })
        .collect();

    store
        .append_batch(&Batch {
            events: stored.clone(),
            ..Batch::default()
        })
        .unwrap();

    assert_eq!(store.events(i64::MIN, i64::MAX).unwrap(), stored);
}

/// One column of one table, as stored, in row order.
fn stored(dir: &Path, sql: &str) -> Vec<i64> {
    let raw = rusqlite::Connection::open(dir.join("vestigium.db")).unwrap();
    let mut query = raw.prepare(sql).unwrap();
    query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn every_event_meaning_is_stored_under_its_own_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let events = EventMeaning::ALL
        .into_iter()
        .zip(1..)
        .map(|(meaning, n)| WinEvent {
            meaning,
            ..event(n * 1_000, n as u64)
        })
        .collect();
    store
        .append_batch(&Batch {
            events,
            ..Batch::default()
        })
        .unwrap();

    let codes = stored(dir.path(), "SELECT meaning FROM event ORDER BY id");

    assert_eq!(codes, (1..=16).collect::<Vec<i64>>());
    assert_eq!(codes.len(), EventMeaning::ALL.len());
}

#[test]
fn every_channel_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let stored_events: Vec<WinEvent> = Channel::ALL
        .into_iter()
        .zip(1..)
        .map(|(channel, n)| WinEvent {
            channel,
            ..event(n * 1_000, n as u64)
        })
        .collect();
    store
        .append_batch(&Batch {
            events: stored_events.clone(),
            ..Batch::default()
        })
        .unwrap();

    assert_eq!(store.events(i64::MIN, i64::MAX).unwrap(), stored_events);
    assert_eq!(
        stored(dir.path(), "SELECT channel FROM event ORDER BY id"),
        [1, 2]
    );
}

#[test]
fn every_gap_reason_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let gaps: Vec<Gap> = GapReason::ALL
        .into_iter()
        .zip(1..)
        .map(|(reason, n)| Gap {
            reason,
            ..gap(n * 1_000, n * 1_000 + 500)
        })
        .collect();
    store
        .append_batch(&Batch {
            gaps: gaps.clone(),
            ..Batch::default()
        })
        .unwrap();

    assert_eq!(store.gaps(i64::MIN, i64::MAX).unwrap(), gaps);
    assert_eq!(
        stored(dir.path(), "SELECT reason FROM gap ORDER BY id"),
        [1, 2, 3, 4, 5]
    );
}

#[test]
fn every_occurrence_kind_and_severity_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let mut expected = Vec::new();
    for (kind, k) in OccurrenceKind::ALL.into_iter().zip(0..) {
        for (severity, s) in Severity::ALL.into_iter().zip(0..) {
            let start_ms = (k * 10 + s) * 1_000;
            let id = store
                .insert_occurrence(&Occurrence {
                    kind,
                    severity,
                    ..occurrence(start_ms, start_ms + 500)
                })
                .unwrap();
            expected.push((id, kind, severity));
        }
    }

    let found: Vec<_> = store
        .occurrences(i64::MIN, i64::MAX)
        .unwrap()
        .into_iter()
        .map(|o| (o.id, o.kind, o.severity))
        .collect();

    assert_eq!(found, expected);
    assert_eq!(
        stored(dir.path(), "SELECT kind FROM occurrence ORDER BY id")
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([1, 2, 3, 4, 5])
    );
    assert_eq!(
        stored(dir.path(), "SELECT severity FROM occurrence ORDER BY id")
            .into_iter()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([1, 2, 3])
    );
}

#[test]
fn a_memory_diagnostic_event_does_not_break_reads() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let stored = vec![event(1_000, 1), memory_diagnostic(2_000, 2)];

    store
        .append_batch(&Batch {
            events: stored.clone(),
            ..Batch::default()
        })
        .unwrap();

    assert_eq!(store.events(0, 10_000).unwrap(), stored);
}

#[test]
fn bookmark_is_written_with_its_events() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let failing = Batch {
        segments: vec![segment(Series::Samples, 1_000, 2_000)],
        events: vec![event(1_500, 7)],
        bookmark_xml: Some("<BookmarkList/>".to_string()),
        gaps: vec![gap(5_000, 4_000)],
    };

    assert!(store.append_batch(&failing).is_err());

    assert!(store.events(0, 10_000).unwrap().is_empty());
    assert_eq!(store.bookmark_xml().unwrap(), None);
    assert!(store.samples(0, 10_000).unwrap().is_empty());

    let good = Batch {
        gaps: vec![gap(4_000, 5_000)],
        ..failing
    };
    store.append_batch(&good).unwrap();
    assert_eq!(store.events(0, 10_000).unwrap().len(), 1);
    assert_eq!(
        store.bookmark_xml().unwrap().as_deref(),
        Some("<BookmarkList/>")
    );
    assert_eq!(store.gaps(0, 10_000).unwrap(), vec![gap(4_000, 5_000)]);
}

#[test]
fn prune_keeps_what_is_inside_retention() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let retention = Retention::default();
    let now = 1_000 * DAY_MS;
    let samples_cutoff = now - 48 * HOUR_MS;
    let minutes_cutoff = now - 90 * DAY_MS;
    let records_cutoff = now - 365 * DAY_MS;
    store
        .append_batch(&Batch {
            segments: vec![
                segment(Series::Samples, samples_cutoff - 10, samples_cutoff - 1),
                segment(Series::Samples, samples_cutoff - 10, samples_cutoff + 1),
                segment(Series::Minutes, minutes_cutoff - 10, minutes_cutoff - 1),
                segment(Series::Minutes, minutes_cutoff - 10, minutes_cutoff + 1),
                segment(Series::Processes, now - HOUR_MS, now),
            ],
            events: vec![event(records_cutoff - 1, 1), event(records_cutoff + 1, 2)],
            bookmark_xml: None,
            gaps: vec![
                gap(records_cutoff - 10, records_cutoff - 1),
                gap(records_cutoff - 10, records_cutoff + 1),
            ],
        })
        .unwrap();
    for (start, end) in [
        (records_cutoff - 10, records_cutoff - 1),
        (records_cutoff - 10, records_cutoff + 1),
    ] {
        store.insert_occurrence(&occurrence(start, end)).unwrap();
        store.insert_episode(&episode(start, end)).unwrap();
    }

    let report = store.prune(now, &retention).unwrap();

    assert_eq!(
        (
            report.segments,
            report.minutes,
            report.occurrences,
            report.episodes,
            report.events,
            report.gaps
        ),
        (1, 1, 1, 1, 1, 1)
    );
    let (all, max) = (i64::MIN, i64::MAX);
    assert_eq!(store.segments(Series::Samples, all, max).unwrap().len(), 1);
    assert_eq!(store.segments(Series::Minutes, all, max).unwrap().len(), 1);
    assert_eq!(store.occurrences(all, max).unwrap().len(), 1);
    assert_eq!(store.episodes(all, max).unwrap().len(), 1);
    assert_eq!(store.events(all, max).unwrap().len(), 1);
    assert_eq!(store.gaps(all, max).unwrap().len(), 1);
}

#[test]
fn prune_refuses_to_run_far_ahead_of_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let now = 1_000 * DAY_MS;
    store
        .append_batch(&Batch {
            segments: vec![segment(Series::Samples, now - HOUR_MS, now)],
            ..Batch::default()
        })
        .unwrap();
    store
        .insert_occurrence(&occurrence(now - 60_000, now))
        .unwrap();

    let report = store
        .prune(now + 3 * DAY_MS, &Retention::default())
        .unwrap();

    assert!(report.refused);
    assert_eq!(report.segments, 0);
    assert!(!store.samples(now - 2 * HOUR_MS, now).unwrap().is_empty());
    assert_eq!(store.occurrences(i64::MIN, i64::MAX).unwrap().len(), 1);
}

#[test]
fn prune_runs_when_now_is_within_an_hour_of_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let now = 1_000 * DAY_MS;
    let old_end = now - 48 * HOUR_MS - 1;
    store
        .append_batch(&Batch {
            segments: vec![
                segment(Series::Samples, old_end - 10, old_end),
                segment(Series::Samples, now - HOUR_MS, now),
            ],
            ..Batch::default()
        })
        .unwrap();

    let report = store.prune(now + HOUR_MS, &Retention::default()).unwrap();

    assert!(!report.refused);
    assert_eq!(report.segments, 1);
}

#[test]
fn prune_runs_when_the_clock_is_behind_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let now = 1_000 * DAY_MS;
    let old_end = now - 48 * HOUR_MS - 1;
    store
        .append_batch(&Batch {
            segments: vec![
                segment(Series::Samples, old_end - 10, old_end),
                segment(Series::Samples, now + 2 * HOUR_MS, now + 3 * HOUR_MS),
            ],
            ..Batch::default()
        })
        .unwrap();

    let report = store.prune(now, &Retention::default()).unwrap();

    assert!(!report.refused);
    assert_eq!(report.segments, 1);
}

#[test]
fn prune_runs_on_a_store_without_segments() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let now = 1_000 * DAY_MS;
    store
        .insert_occurrence(&occurrence(now - 400 * DAY_MS, now - 399 * DAY_MS))
        .unwrap();

    let report = store.prune(now, &Retention::default()).unwrap();

    assert!(!report.refused);
    assert_eq!(report.occurrences, 1);
}

#[test]
fn clear_history_keeps_settings_and_the_bookmark() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let settings = Settings {
        language: LanguagePref::PtBr,
        ..Settings::default()
    };
    store.set_settings(&settings).unwrap();
    store
        .append_batch(&Batch {
            segments: vec![
                segment(Series::Samples, 1_000, 2_000),
                segment(Series::Minutes, 0, 60_000),
            ],
            events: vec![event(1_500, 7)],
            bookmark_xml: Some("<BookmarkList/>".to_string()),
            gaps: vec![gap(4_000, 5_000)],
        })
        .unwrap();
    store.insert_occurrence(&occurrence(1_000, 2_000)).unwrap();
    store.insert_episode(&episode(1_000, 2_000)).unwrap();

    store.clear_history().unwrap();

    let (all, max) = (i64::MIN, i64::MAX);
    assert!(
        store
            .segments(Series::Samples, all, max)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .segments(Series::Minutes, all, max)
            .unwrap()
            .is_empty()
    );
    assert!(store.occurrences(all, max).unwrap().is_empty());
    assert!(store.episodes(all, max).unwrap().is_empty());
    assert!(store.events(all, max).unwrap().is_empty());
    assert!(store.gaps(all, max).unwrap().is_empty());
    assert_eq!(store.settings().unwrap(), settings);
    assert_eq!(
        store.bookmark_xml().unwrap().as_deref(),
        Some("<BookmarkList/>")
    );
}

#[test]
fn read_only_store_refuses_writes() {
    let dir = tempfile::tempdir().unwrap();
    drop(open(dir.path()));
    let mut reader = Store::open_read_only(&dir.path().join("vestigium.db")).unwrap();

    assert!(
        reader
            .append(&[segment(Series::Samples, 1_000, 2_000)])
            .is_err()
    );
    assert!(reader.set_settings(&Settings::default()).is_err());
}

#[test]
fn read_only_open_of_a_missing_db_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");

    assert!(matches!(
        Store::open_read_only(&path),
        Err(StoreError::NotFound)
    ));
    assert!(!path.exists());
}

#[test]
fn read_only_reads_while_the_writer_appends() {
    let dir = tempfile::tempdir().unwrap();
    let mut writer = open(dir.path());
    writer
        .append(&[segment(Series::Samples, 1_000, 1_000)])
        .unwrap();
    let reader = Store::open_read_only(&dir.path().join("vestigium.db")).unwrap();
    assert_eq!(reader.samples(0, 10_000).unwrap().len(), 1);

    writer
        .append(&[segment(Series::Samples, 2_000, 2_000)])
        .unwrap();
    writer.insert_occurrence(&occurrence(1_500, 1_800)).unwrap();

    assert_eq!(reader.samples(0, 10_000).unwrap().len(), 2);
    assert_eq!(reader.occurrences(0, 10_000).unwrap().len(), 1);
    assert_eq!(writer.last_sample_end_ms().unwrap(), Some(2_000));
}

#[test]
fn settings_default_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    assert_eq!(store.settings().unwrap(), Settings::default());
    assert_eq!(store.machine().unwrap(), None);

    let mut changed = Settings::default();
    changed.retention.samples_hours = 72;
    store.set_settings(&changed).unwrap();
    store.set_machine(&machine()).unwrap();

    assert_eq!(store.settings().unwrap(), changed);
    assert_eq!(store.machine().unwrap(), Some(machine()));
}

#[test]
fn settings_default_color_theme_and_detail_level() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let defaults = store.settings().unwrap();
    assert_eq!(defaults.color_theme, ColorTheme::Red);
    assert_eq!(defaults.detail_level, DetailLevel::Simple);

    let changed = Settings {
        color_theme: ColorTheme::Slate,
        detail_level: DetailLevel::Technical,
        ..Settings::default()
    };
    store.set_settings(&changed).unwrap();

    assert_eq!(store.settings().unwrap(), changed);
}

#[test]
fn settings_outside_bounds_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let mut wrong = Settings::default();
    wrong.retention.records_days = 30;

    assert!(matches!(
        store.set_settings(&wrong),
        Err(StoreError::Setting(_))
    ));
    assert_eq!(store.settings().unwrap(), Settings::default());
}

#[test]
fn migration_keeps_one_row_of_each_duplicated_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vestigium.db");
    drop(Store::open(&path).unwrap());
    {
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "DROP INDEX capture_start;
             DROP INDEX scan_ts;
             CREATE INDEX capture_start ON capture (start_ms);
             CREATE INDEX scan_ts ON scan (ts_ms);
             DROP INDEX IF EXISTS tweak_change_key;
             PRAGMA user_version = 10;
             INSERT INTO capture (version, start_ms, end_ms, data) VALUES
                 (2, 1000, 2000, x'01'), (2, 1000, 2000, x'02'), (2, 5000, 6000, x'03');
             INSERT INTO scan (version, ts_ms, data) VALUES
                 (1, 10, x'01'), (1, 10, x'02'), (1, 20, x'03');
             INSERT INTO tweak_change (version, tweak, subject, applied_ms, undone_ms, undo_of,
                                       elevated, data) VALUES
                 (2, 0, NULL, 100, NULL, NULL, 1, x'01'),
                 (2, 0, NULL, 100, 300, NULL, 1, x'02'),
                 (2, 0, 'a', 100, NULL, NULL, 1, x'03'),
                 (2, 0, NULL, 300, NULL, 2, 1, x'04'),
                 (2, 0, NULL, 300, NULL, 1, 1, x'05');",
        )
        .unwrap();
    }

    let store = Store::open(&path).unwrap();

    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    let raw = rusqlite::Connection::open(&path).unwrap();
    let count = |table: &str| -> i64 {
        raw.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    };
    assert_eq!(count("capture"), 2);
    assert_eq!(count("scan"), 2);
    assert_eq!(count("tweak_change"), 3);
    let kept: i64 = raw
        .query_row("SELECT id FROM capture WHERE start_ms = 1000", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(kept, 1, "the lowest id stays");
    let undone: Option<i64> = raw
        .query_row("SELECT undone_ms FROM tweak_change WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(undone, Some(300), "the kept change remembers it was undone");
    let undo_of: i64 = raw
        .query_row(
            "SELECT undo_of FROM tweak_change WHERE applied_ms = 300",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        undo_of, 1,
        "an undo of a removed copy points at the kept one"
    );
}

#[test]
fn a_clock_changed_gap_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    let stepped = Gap {
        reason: GapReason::ClockChanged,
        ..gap(5_000, 5_000)
    };

    store
        .append_batch(&Batch {
            gaps: vec![stepped],
            ..Batch::default()
        })
        .unwrap();

    assert_eq!(store.gaps(0, 10_000).unwrap(), vec![stepped]);
}
