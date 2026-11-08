use std::path::PathBuf;

use serde::Serialize;
use vestigium_core::health::{BootReport, Finding, Measure, Subject};
use vestigium_core::hwtest::{Outcome, TestKind, TestRun, caused_by_test};
use vestigium_core::{
    CaptureCauses, DeepCapture, DumpCulprit, DumpKind, Episode, EpisodeKind, EventMeaning, Fault,
    Gap, MINUTE_MS, MachineInfo, Occurrence, OccurrenceSummary, PcState, ProcessSample, Reading,
    Suspect, WinEvent, bug_check_event, capture_findings, capture_suspects, dump_culprit, heaviest,
    pc_state, steady_only, usage,
};
use vestigium_store::{CaptureSummary, DumpRow, FindingRecord, Store, StoreError, TweakChangeRow};

use crate::hwtests::TestRunDto;

/// One day's reading of a SMART attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SmartPointDto {
    pub read_ms: i64,
    pub current: u8,
    pub worst: u8,
    pub raw: u64,
}

/// How far back the health page lists the findings that closed.
const CLOSED_SHOWN_MS: i64 = 30 * 24 * 3_600_000;
/// How far back the dumps page lists the blue screens Windows reported.
const REPORTS_SHOWN_MS: i64 = 365 * 24 * 3_600_000;

/// Ranges up to this long are drawn from per-second samples, longer ones from minutes.
pub const SECONDS_UP_TO_MS: i64 = 6 * 3_600_000;
const SECOND_MS: i64 = 1_000;
/// Most points a chart gets, whatever the caller asks for.
const MAX_POINTS: u32 = 10_000;
/// Context drawn on each side of an episode: half its length, from a minute to half an hour.
const EPISODE_CONTEXT_MS: (i64, i64) = (60_000, 30 * 60_000);
const EPISODE_POINTS: u32 = 600;
/// Programs an episode lists for each measure.
const EPISODE_PROGRAMS: usize = 5;

/// Whether the watcher has written any history yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryState {
    Ready,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Seconds,
    Minutes,
}

/// A chart's series, one entry per point; `None` where nothing was watched.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Columns {
    /// Start of each point, Unix epoch seconds.
    pub t: Vec<f64>,
    pub cpu: Vec<Option<f32>>,
    pub mem_used_pct: Vec<Option<f32>>,
    pub disk_active_max: Vec<Option<f32>>,
    pub disk_latency_max: Vec<Option<f32>>,
    pub gpu: Vec<Option<f32>>,
    pub probe_lag: Vec<Option<f32>>,
    pub state: Vec<PcState>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TimelineDto {
    pub history: HistoryState,
    pub resolution: Resolution,
    /// Width of each point, in milliseconds.
    pub step_ms: i64,
    #[serde(flatten)]
    pub columns: Columns,
}

/// What the timeline marks: stalls, pressure, Windows events, holes in watching and tests.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarksDto {
    pub history: HistoryState,
    pub occurrences: Vec<OccurrenceSummary>,
    pub episodes: Vec<Episode>,
    /// For each episode, in order: whether a Vestigium test most likely caused it.
    pub by_test: Vec<bool>,
    pub events: Vec<WinEvent>,
    pub gaps: Vec<Gap>,
    pub tests: Vec<TestSpan>,
}

/// A hardware test that ran inside a span, as the timeline and the report show it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TestSpan {
    pub id: i64,
    pub kind: TestKind,
    pub subject: Subject,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub outcome: Outcome,
}

/// Test runs that overlap `[from_ms, to_ms]`, the ones that end later included.
fn test_spans(
    store: &Store,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<(TestSpan, TestRun)>, StoreError> {
    const LONGEST_RUN_MS: i64 = 24 * 3_600_000;
    Ok(store
        .test_runs(from_ms, to_ms + LONGEST_RUN_MS)?
        .into_iter()
        .filter(|row| row.run.started_ms <= to_ms && row.run.ended_ms >= from_ms)
        .map(|row| {
            let span = TestSpan {
                id: row.id,
                kind: row.run.kind(),
                subject: row.run.subject.clone(),
                started_ms: row.run.started_ms,
                ended_ms: row.run.ended_ms,
                outcome: row.run.outcome,
            };
            (span, row.run)
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OccurrenceDto {
    pub id: i64,
    #[serde(flatten)]
    pub occurrence: Occurrence,
    /// The window second by second.
    pub chart: Columns,
    /// What a deep capture that covered this window adds.
    pub capture: Option<CaptureNote>,
    /// Vestigium tests that ran during the window.
    pub tests: Vec<TestSpan>,
}

/// What the watcher holds in memory and has not written yet: its last minutes, a reading a second.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recent {
    pub readings: Vec<Reading>,
    /// Pressure episodes of those minutes, ended or under way, as `(start_ms, end_ms)`.
    pub pressure: Vec<(i64, i64)>,
}

impl Recent {
    /// Adds the readings of `[from_ms, to_ms)` newer than the history's last one, and the pressure.
    fn fill(
        &self,
        readings: &mut Vec<Reading>,
        pressure: &mut Vec<(i64, i64)>,
        from_ms: i64,
        to_ms: i64,
        native_ms: i64,
    ) {
        let stored_end = readings.last().map_or(from_ms, |r| r.ts_ms + native_ms);
        let first_ms = stored_end.max(from_ms);
        readings.extend(
            self.readings
                .iter()
                .filter(|r| r.ts_ms >= first_ms && r.ts_ms < to_ms)
                .copied(),
        );
        pressure.extend(
            self.pressure
                .iter()
                .filter(|&&(start, end)| start < to_ms && end >= from_ms)
                .copied(),
        );
    }
}

/// An episode as its detail shows it: the stretch around it, who used what, what stalled in it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EpisodeDetailDto {
    pub history: HistoryState,
    pub resolution: Resolution,
    pub step_ms: i64,
    /// The stretch drawn: the episode with context on each side, cut at now.
    pub from_ms: i64,
    pub to_ms: i64,
    /// For a disk episode the disk series read that disk alone.
    pub chart: Columns,
    /// Each program's use during the episode; empty once per-second history is gone.
    pub programs: Vec<ProcessSample>,
    /// Occurrences that overlap the episode, oldest first.
    pub occurrences: Vec<OccurrenceSummary>,
}

/// One finding as the health page lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FindingDto {
    pub id: i64,
    pub opened_ms: i64,
    pub updated_ms: i64,
    /// `None` while it still holds.
    pub closed_ms: Option<i64>,
    #[serde(flatten)]
    pub finding: Finding,
    /// What it measured when it opened; `None` for one opened before that was kept.
    pub first_value: Option<Measure>,
}

impl From<FindingRecord> for FindingDto {
    fn from(record: FindingRecord) -> FindingDto {
        FindingDto {
            id: record.id,
            opened_ms: record.opened_ms,
            updated_ms: record.updated_ms,
            closed_ms: record.closed_ms,
            finding: record.finding,
            first_value: record.first.map(|f| f.value),
        }
    }
}

/// What the health page shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct HealthDto {
    pub open: Vec<FindingDto>,
    pub closed: Vec<FindingDto>,
    pub last_scan_ms: Option<i64>,
    /// What the last full scan read from the boot log; `None` without a scan or when it could not.
    pub boot: Option<BootReport>,
}

/// A 64-bit address or parameter as text: a JavaScript number loses its low bits.
fn hex(value: u64) -> String {
    format!("0x{value:016x}")
}

/// What went wrong, with addresses as text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum FaultDto {
    BugCheck { code: u32, params: [String; 4] },
    Exception { code: u32, address: String },
}

impl From<Fault> for FaultDto {
    fn from(fault: Fault) -> FaultDto {
        match fault {
            Fault::BugCheck { code, params } => FaultDto::BugCheck {
                code,
                params: params.map(hex),
            },
            Fault::Exception { code, address } => FaultDto::Exception {
                code,
                address: hex(address),
            },
        }
    }
}

/// One dump as the dumps page lists it, with who it points at.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DumpDto {
    pub id: i64,
    pub path: String,
    pub kind: DumpKind,
    pub crash_ms: i64,
    pub file_ms: i64,
    pub size_bytes: u64,
    pub found_ms: i64,
    /// `None` until the helper read it.
    pub analyzed_ms: Option<i64>,
    /// `None` while the file is still there.
    pub deleted_ms: Option<i64>,
    pub fault: Option<FaultDto>,
    pub fault_module: Option<String>,
    pub stack_modules: Vec<String>,
    pub process: Option<String>,
    pub module_count: Option<u32>,
    pub culprit: Option<DumpCulprit>,
}

impl From<DumpRow> for DumpDto {
    fn from(row: DumpRow) -> DumpDto {
        let analysis = row.record.analysis;
        DumpDto {
            id: row.id,
            path: row.record.path,
            kind: row.record.kind,
            crash_ms: row.record.crash_ms,
            file_ms: row.record.file_ms,
            size_bytes: row.record.size_bytes,
            found_ms: row.found_ms,
            analyzed_ms: row.analyzed_ms,
            deleted_ms: row.deleted_ms,
            culprit: analysis.as_ref().and_then(dump_culprit),
            fault: analysis.as_ref().map(|a| a.fault.into()),
            module_count: analysis.as_ref().map(|a| a.module_count),
            fault_module: analysis.as_ref().and_then(|a| a.fault_module.clone()),
            process: analysis.as_ref().and_then(|a| a.process.clone()),
            stack_modules: analysis.map(|a| a.stack_modules).unwrap_or_default(),
        }
    }
}

/// A blue screen Windows reported on the next boot, read from its event without the dump.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BugCheckDto {
    pub ts_ms: i64,
    pub code: u32,
    pub params: [String; 4],
    /// Where Windows saved the dump; empty when it saved none.
    pub dump_path: String,
}

/// What Windows is set to write when it stops with a blue screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CrashControlDto {
    /// 0 none, 1 complete, 2 kernel, 3 small, 7 automatic.
    pub dump_type: u32,
    pub dump_file: String,
    pub minidump_dir: String,
    pub overwrite: bool,
}

/// What the dumps page shows.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct DumpsDto {
    /// Newest crash first, deleted ones included.
    pub dumps: Vec<DumpDto>,
    /// Bytes the dumps still on disk take.
    pub space_bytes: u64,
    /// Blue screens of the last year, newest first.
    pub reports: Vec<BugCheckDto>,
    /// `None` when the setting cannot be read.
    pub crash: Option<CrashControlDto>,
}

/// A stored deep capture with what it names.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CaptureDto {
    pub id: i64,
    #[serde(flatten)]
    pub capture: DeepCapture,
    pub findings: Vec<Suspect>,
}

/// The suspects a deep capture adds to an occurrence it covered.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CaptureNote {
    pub id: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub suspects: Vec<Suspect>,
}

/// What the tweak rules read from the history: the open findings, the recent occurrences,
/// episodes, captures and events, the machine, and the changes made so far.
#[derive(Clone, Debug, Default)]
pub struct TweakHistory {
    pub findings: Vec<Finding>,
    pub occurrences: Vec<OccurrenceSummary>,
    pub episodes: Vec<Episode>,
    pub captures: Vec<CaptureCauses>,
    pub events: Vec<WinEvent>,
    pub machine: Option<MachineInfo>,
    /// Newest first: those of the shown span and every one still in place.
    pub changes: Vec<TweakChangeRow>,
}

/// The watcher's database, read-only, opened when first needed and again after it goes away.
pub struct History {
    path: Option<PathBuf>,
    store: Option<Store>,
}

impl History {
    pub fn new(path: Option<PathBuf>) -> History {
        History { path, store: None }
    }

    /// `%LOCALAPPDATA%\Vestigium\data\vestigium.db`, where the watcher writes.
    pub fn default_path() -> Option<PathBuf> {
        let local = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty())?;
        Some(PathBuf::from(local).join(r"Vestigium\data\vestigium.db"))
    }

    /// The folder of the watcher's database, where the helper's files go.
    pub fn data_dir(&self) -> Option<PathBuf> {
        self.path.as_deref()?.parent().map(PathBuf::from)
    }

    /// The signals of the last `evidence_ms` and the tweak changes of the last `shown_ms`.
    pub fn tweak_history(
        &mut self,
        now_ms: i64,
        evidence_ms: i64,
        shown_ms: i64,
    ) -> Result<TweakHistory, StoreError> {
        let from = now_ms - evidence_ms;
        let found = self.read(|store| {
            let mut captures = Vec::new();
            for summary in store.captures(from, now_ms)? {
                if let Some(capture) = store.capture(summary.id)? {
                    captures.push(CaptureCauses {
                        start_ms: capture.start_ms,
                        causes: capture_findings(&capture)
                            .into_iter()
                            .map(|s| s.cause)
                            .collect(),
                    });
                }
            }
            let mut changes = store.tweak_changes(now_ms - shown_ms, now_ms)?;
            for row in store.tweaks_in_place()? {
                if !changes.iter().any(|c| c.id == row.id) {
                    changes.push(row);
                }
            }
            changes.sort_by_key(|r| std::cmp::Reverse((r.applied_ms, r.id)));
            Ok(TweakHistory {
                findings: store
                    .open_findings()?
                    .into_iter()
                    .map(|f| f.finding)
                    .collect(),
                occurrences: store.occurrences(from, now_ms)?,
                episodes: store.episodes(from, now_ms)?,
                captures,
                events: store.events(from, now_ms)?,
                machine: store.machine()?,
                changes,
            })
        })?;
        Ok(found.unwrap_or_default())
    }

    pub fn tweak_change(&mut self, id: i64) -> Result<Option<TweakChangeRow>, StoreError> {
        Ok(self.read(|store| store.tweak_change(id))?.flatten())
    }

    /// Points of `[from_ms, to_ms)`, peaks kept, at most `max_points` of them.
    /// `recent` fills what the watcher has not written yet.
    pub fn timeline(
        &mut self,
        from_ms: i64,
        to_ms: i64,
        max_points: u32,
        recent: &Recent,
    ) -> Result<TimelineDto, StoreError> {
        let resolution = if to_ms - from_ms <= SECONDS_UP_TO_MS {
            Resolution::Seconds
        } else {
            Resolution::Minutes
        };
        let native_ms = match resolution {
            Resolution::Seconds => SECOND_MS,
            Resolution::Minutes => MINUTE_MS,
        };
        let start_ms = from_ms - from_ms.rem_euclid(native_ms);
        let step_ms = step(to_ms - start_ms, native_ms, max_points);
        let found = self.read(|store| {
            let last_ms = to_ms.saturating_sub(1);
            let readings: Vec<Reading> = match resolution {
                Resolution::Seconds => store
                    .samples(start_ms, last_ms)?
                    .iter()
                    .map(Reading::of_sample)
                    .collect(),
                Resolution::Minutes => store
                    .minutes(start_ms, last_ms)?
                    .iter()
                    .map(Reading::of_minute)
                    .collect(),
            };
            let pressure: Vec<(i64, i64)> = store
                .episodes(start_ms, last_ms)?
                .iter()
                .filter(|e| !e.kind.is_hint())
                .map(|e| (e.start_ms, e.end_ms))
                .collect();
            Ok((readings, pressure))
        })?;
        let Some((mut readings, mut pressure)) = found else {
            return Ok(TimelineDto {
                history: HistoryState::Empty,
                resolution,
                step_ms,
                columns: Columns::default(),
            });
        };
        recent.fill(&mut readings, &mut pressure, start_ms, to_ms, native_ms);
        let buckets = bucket(&readings, start_ms, to_ms, step_ms);
        Ok(TimelineDto {
            history: HistoryState::Ready,
            resolution,
            step_ms,
            columns: columns(&buckets, start_ms, step_ms, &pressure),
        })
    }

    pub fn marks(&mut self, from_ms: i64, to_ms: i64) -> Result<MarksDto, StoreError> {
        let found = self.read(|store| {
            let episodes = store.episodes(from_ms, to_ms)?;
            let (tests, runs): (Vec<TestSpan>, Vec<TestRun>) =
                test_spans(store, from_ms, to_ms)?.into_iter().unzip();
            Ok(MarksDto {
                history: HistoryState::Ready,
                occurrences: store.occurrences(from_ms, to_ms)?,
                by_test: episodes.iter().map(|e| caused_by_test(e, &runs)).collect(),
                episodes,
                events: store.events(from_ms, to_ms)?,
                gaps: store.gaps(from_ms, to_ms)?,
                tests,
            })
        })?;
        Ok(found.unwrap_or(MarksDto {
            history: HistoryState::Empty,
            occurrences: Vec::new(),
            episodes: Vec::new(),
            by_test: Vec::new(),
            events: Vec::new(),
            gaps: Vec::new(),
            tests: Vec::new(),
        }))
    }

    pub fn occurrence(&mut self, id: i64) -> Result<Option<OccurrenceDto>, StoreError> {
        let found = self.read(|store| {
            let Some(occurrence) = store.occurrence(id)? else {
                return Ok(None);
            };
            let (from, to) = (occurrence.window.from_ms, occurrence.window.to_ms);
            let mut note = None;
            for summary in store.captures(from, to)? {
                if let Some(capture) = store.capture(summary.id)? {
                    note = Some(CaptureNote {
                        id: summary.id,
                        start_ms: summary.start_ms,
                        end_ms: summary.end_ms,
                        suspects: capture_suspects(from, to, &capture),
                    });
                    break;
                }
            }
            let tests = test_spans(store, from, to)?
                .into_iter()
                .map(|(span, _)| span)
                .collect();
            Ok(Some((occurrence, note, tests)))
        })?;
        Ok(found.flatten().map(|(occurrence, capture, tests)| {
            let window = &occurrence.window;
            let readings: Vec<Reading> = window.samples.iter().map(Reading::of_sample).collect();
            let start_ms = window.from_ms - window.from_ms.rem_euclid(SECOND_MS);
            let buckets = bucket(&readings, start_ms, window.to_ms + 1, SECOND_MS);
            OccurrenceDto {
                id,
                chart: columns(&buckets, start_ms, SECOND_MS, &[]),
                occurrence,
                capture,
                tests,
            }
        }))
    }

    /// The detail of `episode`, read from per-second samples while they are kept, else minutes;
    /// `recent` fills what the watcher has not written yet.
    pub fn episode(
        &mut self,
        episode: &Episode,
        now_ms: i64,
        recent: &Recent,
    ) -> Result<EpisodeDetailDto, StoreError> {
        let (least, most) = EPISODE_CONTEXT_MS;
        let context = ((episode.end_ms - episode.start_ms) / 2).clamp(least, most);
        let from_ms = episode.start_ms - context;
        let to_ms = (episode.end_ms + context)
            .min(now_ms)
            .max(episode.end_ms + SECOND_MS);
        let disk = match &episode.kind {
            EpisodeKind::DiskBusy { disk } | EpisodeKind::DiskLatencyHint { disk } => {
                Some(disk.as_str())
            }
            _ => None,
        };
        let found = self.read(|store| {
            let last_ms = to_ms - 1;
            let samples = store.samples(from_ms, last_ms)?;
            let (resolution, readings): (Resolution, Vec<Reading>) = if samples.is_empty() {
                let minutes = store.minutes(from_ms, last_ms)?;
                let readings = minutes.iter().map(|m| match disk {
                    Some(disk) => Reading::of_minute_on(m, disk),
                    None => Reading::of_minute(m),
                });
                (Resolution::Minutes, readings.collect())
            } else {
                let readings = samples.iter().map(|s| match disk {
                    Some(disk) => Reading::of_sample_on(s, disk),
                    None => Reading::of_sample(s),
                });
                (Resolution::Seconds, readings.collect())
            };
            let pressure: Vec<(i64, i64)> = store
                .episodes(from_ms, last_ms)?
                .iter()
                .filter(|e| !e.kind.is_hint())
                .map(|e| (e.start_ms, e.end_ms))
                .collect();
            let tops = store.tops(episode.start_ms, episode.end_ms)?;
            let occurrences = store.occurrences(episode.start_ms, episode.end_ms)?;
            Ok((resolution, readings, pressure, tops, occurrences))
        })?;
        let Some((resolution, mut readings, mut pressure, tops, occurrences)) = found else {
            return Ok(EpisodeDetailDto {
                history: HistoryState::Empty,
                resolution: Resolution::Seconds,
                step_ms: SECOND_MS,
                from_ms,
                to_ms,
                chart: Columns::default(),
                programs: Vec::new(),
                occurrences: Vec::new(),
            });
        };
        let native_ms = match resolution {
            Resolution::Seconds => SECOND_MS,
            Resolution::Minutes => MINUTE_MS,
        };
        let start_ms = from_ms - from_ms.rem_euclid(native_ms);
        let step_ms = step(to_ms - start_ms, native_ms, EPISODE_POINTS);
        recent.fill(&mut readings, &mut pressure, start_ms, to_ms, native_ms);
        let buckets = bucket(&readings, start_ms, to_ms, step_ms);
        Ok(EpisodeDetailDto {
            history: HistoryState::Ready,
            resolution,
            step_ms,
            from_ms,
            to_ms,
            chart: columns(&buckets, start_ms, step_ms, &pressure),
            programs: heaviest(usage(&tops), EPISODE_PROGRAMS),
            occurrences,
        })
    }

    /// Deep captures that overlap `[from_ms, to_ms]`, oldest first.
    pub fn captures(
        &mut self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<CaptureSummary>, StoreError> {
        Ok(self
            .read(|store| store.captures(from_ms, to_ms))?
            .unwrap_or_default())
    }

    pub fn capture(&mut self, id: i64) -> Result<Option<CaptureDto>, StoreError> {
        let found = self.read(|store| store.capture(id))?;
        Ok(found.flatten().map(|capture| {
            let capture = steady_only(&capture);
            CaptureDto {
                id,
                findings: capture_findings(&capture),
                capture,
            }
        }))
    }

    /// The open findings, the critical first, the ones that closed in the last 30 days, newest
    /// first, and what the last full scan read.
    pub fn health(&mut self, now_ms: i64) -> Result<HealthDto, StoreError> {
        let read = self.read(|store| {
            Ok((
                store.findings(now_ms - CLOSED_SHOWN_MS, now_ms)?,
                store.last_scan()?,
            ))
        })?;
        let Some((records, scan)) = read else {
            return Ok(HealthDto::default());
        };
        let (mut open, mut closed): (Vec<_>, Vec<_>) = records
            .into_iter()
            .map(FindingDto::from)
            .partition(|f| f.closed_ms.is_none());
        open.sort_by_key(|f| (std::cmp::Reverse(f.finding.concern), f.opened_ms));
        closed.sort_by_key(|f| std::cmp::Reverse(f.closed_ms));
        Ok(HealthDto {
            open,
            closed,
            last_scan_ms: scan.as_ref().map(|s| s.ts_ms),
            boot: scan.and_then(|s| s.boot),
        })
    }

    /// The dumps Windows kept, what each says, the space they take and the blue screens Windows
    /// reported in the last year.
    pub fn dumps(&mut self, now_ms: i64) -> Result<DumpsDto, StoreError> {
        let read = self.read(|store| {
            Ok((
                store.dumps(i64::MIN, i64::MAX)?,
                store.dump_space()?,
                store.events(now_ms - REPORTS_SHOWN_MS, now_ms)?,
            ))
        })?;
        let Some((rows, space_bytes, events)) = read else {
            return Ok(DumpsDto::default());
        };
        let mut reports: Vec<BugCheckDto> = events
            .iter()
            .filter(|e| e.meaning == EventMeaning::BugCheck)
            .filter_map(|e| {
                bug_check_event(&e.data_xml).map(|r| BugCheckDto {
                    ts_ms: e.ts_ms,
                    code: r.code,
                    params: r.params.map(hex),
                    dump_path: r.dump_path,
                })
            })
            .collect();
        reports.reverse();
        Ok(DumpsDto {
            dumps: rows.into_iter().map(DumpDto::from).collect(),
            space_bytes,
            reports,
            crash: None,
        })
    }

    /// The paths of the dumps with these ids that are still on disk.
    pub fn dump_paths(&mut self, ids: &[i64]) -> Result<Vec<String>, StoreError> {
        let rows = self
            .read(|store| store.present_dumps())?
            .unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter(|row| ids.contains(&row.id))
            .map(|row| row.record.path)
            .collect())
    }

    pub fn settings(&mut self) -> Result<Option<vestigium_core::Settings>, StoreError> {
        self.read(|store| store.settings())
    }

    pub fn machine(&mut self) -> Result<Option<MachineInfo>, StoreError> {
        Ok(self.read(|store| store.machine())?.flatten())
    }

    /// The daily readings of one SMART attribute of a disk since `from_ms`, oldest first.
    pub fn smart_history(
        &mut self,
        disk: &str,
        attr: u8,
        from_ms: i64,
    ) -> Result<Vec<SmartPointDto>, StoreError> {
        let rows = self.read(|store| store.smart_history(disk, attr, from_ms))?;
        Ok(rows
            .unwrap_or_default()
            .into_iter()
            .map(|r| SmartPointDto {
                read_ms: r.read_ms,
                current: r.current,
                worst: r.worst,
                raw: r.raw,
            })
            .collect())
    }

    /// Test runs that ended in `[from_ms, to_ms]`, oldest first.
    pub fn test_runs(&mut self, from_ms: i64, to_ms: i64) -> Result<Vec<TestRunDto>, StoreError> {
        let rows = self.read(|store| store.test_runs(from_ms, to_ms))?;
        Ok(rows
            .unwrap_or_default()
            .into_iter()
            .map(|row| TestRunDto {
                id: row.id,
                run: row.run,
            })
            .collect())
    }

    /// Runs `query` on the open database; `None` while there is no database yet.
    fn read<T>(
        &mut self,
        query: impl FnOnce(&Store) -> Result<T, StoreError>,
    ) -> Result<Option<T>, StoreError> {
        let Some(path) = self.path.as_deref().filter(|p| p.is_file()) else {
            self.store = None;
            return Ok(None);
        };
        if self.store.is_none() {
            match Store::open_read_only(path) {
                Ok(store) => self.store = Some(store),
                Err(StoreError::NotFound) => return Ok(None),
                Err(err) => return Err(err),
            }
        }
        let Some(store) = self.store.as_ref() else {
            return Ok(None);
        };
        query(store).map(Some).inspect_err(|_| self.store = None)
    }
}

/// Smallest whole multiple of `native_ms` that fits `span_ms` in `max_points` points.
fn step(span_ms: i64, native_ms: i64, max_points: u32) -> i64 {
    let points = i64::from(max_points.clamp(1, MAX_POINTS));
    let wanted = (span_ms.max(1) + points - 1) / points;
    ((wanted + native_ms - 1) / native_ms).max(1) * native_ms
}

/// Peaks of `readings` in buckets of `step_ms` from `start_ms` to `end_ms`, empty buckets `None`.
fn bucket(readings: &[Reading], start_ms: i64, end_ms: i64, step_ms: i64) -> Vec<Option<Reading>> {
    let count = usize::try_from((end_ms - start_ms + step_ms - 1) / step_ms).unwrap_or(0);
    let mut buckets: Vec<Option<Reading>> = vec![None; count];
    for reading in readings {
        let Ok(at) = usize::try_from((reading.ts_ms - start_ms).div_euclid(step_ms)) else {
            continue;
        };
        match buckets.get_mut(at) {
            Some(Some(peak)) => peak.peak(reading),
            Some(empty) => {
                *empty = Some(Reading {
                    ts_ms: start_ms + at as i64 * step_ms,
                    ..*reading
                })
            }
            None => {}
        }
    }
    buckets
}

/// Columns for `buckets`; a bucket overlapping a span in `pressure` is under pressure.
fn columns(
    buckets: &[Option<Reading>],
    start_ms: i64,
    step_ms: i64,
    pressure: &[(i64, i64)],
) -> Columns {
    let mut columns = Columns::default();
    for (at, reading) in buckets.iter().enumerate() {
        let from_ms = start_ms + at as i64 * step_ms;
        let to_ms = from_ms + step_ms;
        let in_episode = pressure.iter().any(|&(s, e)| s < to_ms && e >= from_ms);
        columns.t.push(from_ms as f64 / 1_000.0);
        columns.cpu.push(reading.map(|r| r.cpu_pct));
        columns
            .mem_used_pct
            .push(reading.and_then(|r| r.mem_used_pct));
        columns
            .disk_active_max
            .push(reading.map(|r| r.disk_active_pct));
        columns
            .disk_latency_max
            .push(reading.map(|r| r.disk_latency_ms));
        columns.gpu.push(reading.and_then(|r| r.gpu_pct));
        columns.probe_lag.push(reading.map(|r| r.probe_lag_ms));
        columns.state.push(pc_state(reading.as_ref(), in_episode));
    }
    columns
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use vestigium_core::{
        Cause, Confidence, CoreLoad, DiskLoad, Evidence, MachineInfo, Metric, MinuteSample,
        OccurrenceKind, ProcessTop, Sample, Severity, Suspect, Trigger, Window,
    };
    use vestigium_store::{Segment, Series, encode};

    use super::*;

    const HOUR_MS: i64 = 3_600_000;
    /// A fixed day, so tests never depend on the clock.
    const DAY_START_MS: i64 = 1_790_000_000_000 - 1_790_000_000_000 % (24 * HOUR_MS);

    fn sample(ts_ms: i64, cpu_pct: f32) -> Sample {
        Sample {
            ts_ms,
            cpu_pct,
            dpc_pct: 0.2,
            isr_pct: 0.1,
            perf_limit_pct: 100.0,
            proc_perf_pct: 100.0,
            mem_total_bytes: 16 << 30,
            mem_avail_bytes: 8 << 30,
            commit_bytes: 8 << 30,
            commit_limit_bytes: 32 << 30,
            hard_faults_per_s: 0.0,
            gpu_pct: Some(2.0),
            probe_max_lag_ms: 1.0,
            sampler_lag_ms: 1.0,
            cores: vec![
                CoreLoad {
                    cpu_pct,
                    dpc_pct: 0.2,
                    isr_pct: 0.1,
                };
                2
            ],
            disks: vec![DiskLoad {
                name: "0 C:".to_string(),
                active_pct: 4.0,
                latency_ms: 1.0,
                queue_len: 0.0,
                bytes_per_s: 0.0,
            }],
        }
    }

    fn minute(ts_ms: i64, cpu_max: f32) -> MinuteSample {
        vestigium_core::summarize_minute(&[sample(ts_ms, cpu_max)]).unwrap()
    }

    /// Writes `samples` the way the watcher does: one segment per minute.
    fn write_samples(store: &mut Store, samples: &[Sample]) {
        let segments: Vec<Segment> = samples
            .chunk_by(|a, b| a.ts_ms / MINUTE_MS == b.ts_ms / MINUTE_MS)
            .map(|chunk| Segment {
                series: Series::Samples,
                start_ms: chunk[0].ts_ms,
                end_ms: chunk[chunk.len() - 1].ts_ms,
                data: encode(chunk),
            })
            .collect();
        store.append(&segments).unwrap();
    }

    fn history(dir: &Path) -> (Store, History) {
        let path = dir.join("vestigium.db");
        (Store::open(&path).unwrap(), History::new(Some(path)))
    }

    #[test]
    fn downsample_keeps_peaks() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let spike_ms = DAY_START_MS + 4 * HOUR_MS + 1_234_000;
        let samples: Vec<Sample> = (0..6 * 3_600)
            .map(|s| DAY_START_MS + s * SECOND_MS)
            .map(|ts| sample(ts, if ts == spike_ms { 100.0 } else { 5.0 }))
            .collect();
        write_samples(&mut store, &samples);

        let timeline = history
            .timeline(
                DAY_START_MS,
                DAY_START_MS + 6 * HOUR_MS,
                600,
                &Recent::default(),
            )
            .unwrap();

        assert_eq!(timeline.resolution, Resolution::Seconds);
        assert_eq!(timeline.step_ms, 36_000);
        assert_eq!(timeline.columns.t.len(), 600);
        let cpu = &timeline.columns.cpu;
        assert_eq!(cpu.iter().flatten().fold(0.0, |a: f32, &b| a.max(b)), 100.0);
        let at = ((spike_ms - DAY_START_MS) / 36_000) as usize;
        assert_eq!(cpu[at], Some(100.0));
        assert_eq!(timeline.columns.state[at], PcState::Busy);
    }

    #[test]
    fn gaps_become_nulls() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let samples: Vec<Sample> = (0..60)
            .chain(120..180)
            .map(|s| sample(DAY_START_MS + s * SECOND_MS, 10.0))
            .collect();
        write_samples(&mut store, &samples);

        let timeline = history
            .timeline(
                DAY_START_MS,
                DAY_START_MS + 180_000,
                1_000,
                &Recent::default(),
            )
            .unwrap();

        assert_eq!(timeline.step_ms, SECOND_MS);
        let cpu = &timeline.columns.cpu;
        assert_eq!(cpu.len(), 180);
        assert_eq!(cpu[59], Some(10.0));
        assert!(cpu[60..120].iter().all(Option::is_none));
        assert!(
            timeline.columns.mem_used_pct[60..120]
                .iter()
                .all(Option::is_none)
        );
        assert_eq!(timeline.columns.state[90], PcState::Suspended);
        assert_eq!(cpu[120], Some(10.0));
        assert_eq!(timeline.columns.state[120], PcState::Calm);
    }

    #[test]
    fn long_ranges_use_minutes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let minutes: Vec<MinuteSample> = (0..24 * 60)
            .map(|m| {
                minute(
                    DAY_START_MS + m * MINUTE_MS,
                    if m == 600 { 70.0 } else { 3.0 },
                )
            })
            .collect();
        store
            .append(&[Segment {
                series: Series::Minutes,
                start_ms: DAY_START_MS,
                end_ms: DAY_START_MS + 24 * HOUR_MS - MINUTE_MS,
                data: encode(&minutes),
            }])
            .unwrap();
        store
            .insert_episode(&Episode {
                kind: EpisodeKind::MemoryHigh,
                start_ms: DAY_START_MS + 2 * HOUR_MS,
                end_ms: DAY_START_MS + 2 * HOUR_MS + 90_000,
                peak: 96.0,
                culprit: None,
                truncated: false,
            })
            .unwrap();

        let timeline = history
            .timeline(
                DAY_START_MS,
                DAY_START_MS + 24 * HOUR_MS,
                2_000,
                &Recent::default(),
            )
            .unwrap();

        assert_eq!(timeline.resolution, Resolution::Minutes);
        assert_eq!(timeline.step_ms, MINUTE_MS);
        assert_eq!(timeline.columns.t.len(), 24 * 60);
        assert_eq!(
            timeline.columns.t[1],
            (DAY_START_MS + MINUTE_MS) as f64 / 1_000.0
        );
        assert_eq!(timeline.columns.cpu[600], Some(70.0));
        assert_eq!(timeline.columns.state[600], PcState::Busy);
        assert_eq!(timeline.columns.state[120], PcState::Pressure);
        assert_eq!(timeline.columns.state[121], PcState::Pressure);
        assert_eq!(timeline.columns.state[123], PcState::Calm);
    }

    #[test]
    fn empty_db_is_an_empty_timeline() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = History::new(Some(dir.path().join("vestigium.db")));

        let timeline = history
            .timeline(
                DAY_START_MS,
                DAY_START_MS + HOUR_MS,
                500,
                &Recent::default(),
            )
            .unwrap();
        let marks = history.marks(DAY_START_MS, DAY_START_MS + HOUR_MS).unwrap();

        assert_eq!(timeline.history, HistoryState::Empty);
        assert!(timeline.columns.t.is_empty());
        assert_eq!(marks.history, HistoryState::Empty);
        assert!(marks.occurrences.is_empty());
        assert_eq!(history.occurrence(1).unwrap(), None);
        assert!(!dir.path().join("vestigium.db").exists());
    }

    #[test]
    fn history_appears_once_the_watcher_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vestigium.db");
        let mut history = History::new(Some(path.clone()));
        assert_eq!(history.marks(0, 1).unwrap().history, HistoryState::Empty);

        let mut store = Store::open(&path).unwrap();
        write_samples(&mut store, &[sample(DAY_START_MS, 1.0)]);

        let timeline = history
            .timeline(DAY_START_MS, DAY_START_MS + 60_000, 60, &Recent::default())
            .unwrap();
        assert_eq!(timeline.history, HistoryState::Ready);
        assert_eq!(timeline.columns.cpu[0], Some(1.0));
    }

    #[test]
    fn occurrence_dto_keeps_evidence_values() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let start_ms = DAY_START_MS + HOUR_MS;
        let evidence = Evidence::Metric {
            ts_ms: start_ms + 500,
            metric: Metric::DiskLatencyMs,
            value: 1_840.5,
            threshold: 200.0,
            subject: Some("1 D:".to_string()),
        };
        let occurrence = Occurrence {
            kind: OccurrenceKind::Freeze,
            start_ms,
            end_ms: start_ms + 6_000,
            severity: Severity::Severe,
            triggers: vec![Trigger::Freeze {
                start_ms,
                end_ms: start_ms + 6_000,
                lag_ms: 6_000.0,
            }],
            window: Window {
                from_ms: start_ms - 60_000,
                to_ms: start_ms + 36_000,
                samples: vec![
                    sample(start_ms - 2_000, 30.0),
                    sample(start_ms + 7_000, 95.0),
                ],
                tops: Vec::new(),
                events: Vec::new(),
                hangs: Vec::new(),
                gaps: Vec::new(),
                machine: MachineInfo {
                    logical_cpus: 2,
                    mem_total_bytes: 16 << 30,
                    disks: Vec::new(),
                },
            },
            suspects: vec![Suspect {
                cause: Cause::Storage {
                    disk: Some("1 D:".to_string()),
                },
                confidence: Confidence::High,
                evidence: vec![evidence.clone()],
                next_steps: Vec::new(),
            }],
            facts: Vec::new(),
            sensitive: false,
            truncated: false,
        };
        let id = store.insert_occurrence(&occurrence).unwrap();

        let dto = history.occurrence(id).unwrap().unwrap();

        assert_eq!(dto.id, id);
        assert_eq!(dto.occurrence.suspects[0].evidence, vec![evidence]);
        assert_eq!(dto.chart.t.len(), 97);
        assert_eq!(dto.chart.cpu[58], Some(30.0));
        assert_eq!(dto.chart.cpu[59], None);
        assert_eq!(dto.chart.cpu[67], Some(95.0));
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(
            json["suspects"][0]["evidence"][0]["Metric"]["value"],
            serde_json::json!(1_840.5)
        );
        assert_eq!(json["id"], serde_json::json!(id));
    }

    fn disk_sample(ts_ms: i64, c_active: f32, d_active: f32) -> Sample {
        let disk = |name: &str, active_pct: f32, latency_ms: f32| DiskLoad {
            name: name.to_string(),
            active_pct,
            latency_ms,
            queue_len: 0.0,
            bytes_per_s: 0.0,
        };
        Sample {
            disks: vec![disk("0 C:", c_active, 2.0), disk("1 D:", d_active, 9.0)],
            ..sample(ts_ms, 10.0)
        }
    }

    fn top(ts_ms: i64, name: &str, cpu_pct: f32, io_bps: f32) -> ProcessTop {
        ProcessTop {
            ts_ms,
            entries: vec![ProcessSample {
                pid: 40,
                name: name.to_string(),
                cpu_pct,
                private_bytes: 64 << 20,
                read_bps: io_bps,
                write_bps: 0.0,
                gpu_pct: 0.0,
            }],
        }
    }

    fn disk_episode(start_ms: i64, end_ms: i64) -> Episode {
        Episode {
            kind: EpisodeKind::DiskBusy {
                disk: "0 C:".to_string(),
            },
            start_ms,
            end_ms,
            peak: 100.0,
            culprit: None,
            truncated: false,
        }
    }

    /// A short freeze at `start_ms` with an empty window.
    fn freeze_at(start_ms: i64) -> Occurrence {
        Occurrence {
            kind: OccurrenceKind::Freeze,
            start_ms,
            end_ms: start_ms + 4_000,
            severity: Severity::Medium,
            triggers: Vec::new(),
            window: Window {
                from_ms: start_ms - 60_000,
                to_ms: start_ms + 30_000,
                samples: Vec::new(),
                tops: Vec::new(),
                events: Vec::new(),
                hangs: Vec::new(),
                gaps: Vec::new(),
                machine: MachineInfo {
                    logical_cpus: 2,
                    mem_total_bytes: 16 << 30,
                    disks: Vec::new(),
                },
            },
            suspects: Vec::new(),
            facts: Vec::new(),
            sensitive: false,
            truncated: false,
        }
    }

    #[test]
    fn episode_detail_draws_the_episode_disk_with_context() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let start_ms = DAY_START_MS + HOUR_MS;
        let end_ms = start_ms + 120_000;
        let samples: Vec<Sample> = (-30..180)
            .map(|s| start_ms + s * SECOND_MS)
            .map(|ts| {
                let busy = ts >= start_ms && ts <= end_ms;
                disk_sample(ts, if busy { 100.0 } else { 5.0 }, 60.0)
            })
            .collect();
        write_samples(&mut store, &samples);
        let tops: Vec<ProcessTop> = (0..=24)
            .map(|i| top(start_ms + i * 5_000, "download.exe", 2.0, 40e6))
            .collect();
        store
            .append(&[Segment {
                series: Series::Processes,
                start_ms,
                end_ms,
                data: encode(&tops),
            }])
            .unwrap();
        let episode = disk_episode(start_ms, end_ms);
        store.insert_episode(&episode).unwrap();

        let detail = history
            .episode(&episode, end_ms + HOUR_MS, &Recent::default())
            .unwrap();

        assert_eq!(detail.history, HistoryState::Ready);
        assert_eq!(detail.resolution, Resolution::Seconds);
        assert_eq!(
            (detail.from_ms, detail.to_ms),
            (start_ms - 60_000, end_ms + 60_000)
        );
        assert_eq!(detail.step_ms, SECOND_MS);
        let disk = &detail.chart.disk_active_max;
        assert_eq!(disk.len(), 240);
        assert_eq!(disk[0], None, "nothing was watched a minute before");
        assert_eq!(disk[40], Some(5.0), "C: alone, not the busier D:");
        assert_eq!(disk[90], Some(100.0));
        assert_eq!(detail.chart.state[90], PcState::Pressure);
        assert_eq!(detail.programs[0].name, "download.exe");
        assert_eq!(detail.programs[0].read_bps, 40e6);
    }

    #[test]
    fn episode_detail_lists_what_stalled_inside_it() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let start_ms = DAY_START_MS + HOUR_MS;
        let end_ms = start_ms + 120_000;
        write_samples(&mut store, &[disk_sample(start_ms, 100.0, 0.0)]);
        for at in [start_ms + 30_000, end_ms + 600_000] {
            store.insert_occurrence(&freeze_at(at)).unwrap();
        }

        let detail = history
            .episode(
                &disk_episode(start_ms, end_ms),
                end_ms + HOUR_MS,
                &Recent::default(),
            )
            .unwrap();

        let starts: Vec<i64> = detail.occurrences.iter().map(|o| o.start_ms).collect();
        assert_eq!(starts, [start_ms + 30_000]);
    }

    #[test]
    fn episode_detail_stops_at_now_and_falls_back_to_minutes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let start_ms = DAY_START_MS + 2 * HOUR_MS;
        let end_ms = start_ms + 2 * HOUR_MS;
        let minutes: Vec<MinuteSample> = (0..5 * 60)
            .map(|m| minute(DAY_START_MS + HOUR_MS + m * MINUTE_MS, 97.0))
            .collect();
        store
            .append(&[Segment {
                series: Series::Minutes,
                start_ms: DAY_START_MS + HOUR_MS,
                end_ms: DAY_START_MS + 6 * HOUR_MS,
                data: encode(&minutes),
            }])
            .unwrap();
        let episode = Episode {
            kind: EpisodeKind::CpuSaturated,
            start_ms,
            end_ms,
            peak: 100.0,
            culprit: None,
            truncated: false,
        };

        let detail = history
            .episode(&episode, end_ms + 10 * 60_000, &Recent::default())
            .unwrap();

        assert_eq!(detail.resolution, Resolution::Minutes);
        assert_eq!(
            detail.from_ms,
            start_ms - 30 * 60_000,
            "half an hour at most"
        );
        assert_eq!(detail.to_ms, end_ms + 10 * 60_000, "never past now");
        assert_eq!(detail.chart.cpu[31], Some(97.0));
        assert!(
            detail.programs.is_empty(),
            "rankings are kept with the seconds"
        );
    }

    #[test]
    fn episode_detail_without_history_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut history = History::new(Some(dir.path().join("vestigium.db")));
        let episode = disk_episode(DAY_START_MS, DAY_START_MS + 60_000);

        let detail = history
            .episode(&episode, DAY_START_MS + HOUR_MS, &Recent::default())
            .unwrap();

        assert_eq!(detail.history, HistoryState::Empty);
        assert!(detail.chart.t.is_empty());
    }

    #[test]
    fn the_watchers_minutes_fill_what_is_not_written_yet() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, mut history) = history(dir.path());
        let written: Vec<Sample> = (0..120)
            .map(|s| sample(DAY_START_MS + s * SECOND_MS, 10.0))
            .collect();
        write_samples(&mut store, &written);
        let recent = Recent {
            readings: (60..300)
                .map(|s| Reading {
                    cpu_pct: 70.0,
                    ..Reading::of_sample(&sample(DAY_START_MS + s * SECOND_MS, 0.0))
                })
                .collect(),
            pressure: vec![(DAY_START_MS + 200_000, DAY_START_MS + 260_000)],
        };

        let timeline = history
            .timeline(DAY_START_MS, DAY_START_MS + 300_000, 1_000, &recent)
            .unwrap();

        let cpu = &timeline.columns.cpu;
        assert_eq!(cpu.len(), 300);
        assert_eq!(
            cpu[100],
            Some(10.0),
            "the history wins where it has the second"
        );
        assert_eq!(cpu[150], Some(70.0), "the watcher fills after it");
        assert_eq!(cpu[299], Some(70.0));
        assert_eq!(timeline.columns.state[230], PcState::Pressure);
        assert_eq!(timeline.columns.state[150], PcState::Busy);
    }

    #[test]
    fn step_fits_the_points_in_whole_units() {
        assert_eq!(step(HOUR_MS, SECOND_MS, 1_000), 4_000);
        assert_eq!(step(HOUR_MS, SECOND_MS, 0), HOUR_MS);
        assert_eq!(step(60_000, SECOND_MS, 1_000_000), SECOND_MS);
        assert_eq!(step(24 * HOUR_MS, MINUTE_MS, 600), 3 * MINUTE_MS);
    }
}
