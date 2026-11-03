use std::path::PathBuf;

use serde::Serialize;
use vestigium_core::health::{BootReport, Finding};
use vestigium_core::{
    CaptureCauses, DeepCapture, DumpCulprit, DumpKind, Episode, EventMeaning, Fault, Gap,
    MINUTE_MS, MachineInfo, Occurrence, OccurrenceSummary, PcState, Reading, Suspect, WinEvent,
    bug_check_event, capture_findings, capture_suspects, dump_culprit, pc_state, steady_only,
};
use vestigium_store::{CaptureSummary, DumpRow, FindingRecord, Store, StoreError, TweakChangeRow};

/// How far back the health page lists the findings that closed.
const CLOSED_SHOWN_MS: i64 = 30 * 24 * 3_600_000;
/// How far back the dumps page lists the blue screens Windows reported.
const REPORTS_SHOWN_MS: i64 = 365 * 24 * 3_600_000;

/// Ranges up to this long are drawn from per-second samples, longer ones from minutes.
pub const SECONDS_UP_TO_MS: i64 = 6 * 3_600_000;
const SECOND_MS: i64 = 1_000;
/// Most points a chart gets, whatever the caller asks for.
const MAX_POINTS: u32 = 10_000;

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

/// What the timeline marks: stalls, pressure, Windows events and holes in watching.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MarksDto {
    pub history: HistoryState,
    pub occurrences: Vec<OccurrenceSummary>,
    pub episodes: Vec<Episode>,
    pub events: Vec<WinEvent>,
    pub gaps: Vec<Gap>,
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
}

impl From<FindingRecord> for FindingDto {
    fn from(record: FindingRecord) -> FindingDto {
        FindingDto {
            id: record.id,
            opened_ms: record.opened_ms,
            updated_ms: record.updated_ms,
            closed_ms: record.closed_ms,
            finding: record.finding,
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
    pub fn timeline(
        &mut self,
        from_ms: i64,
        to_ms: i64,
        max_points: u32,
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
        let Some((readings, pressure)) = found else {
            return Ok(TimelineDto {
                history: HistoryState::Empty,
                resolution,
                step_ms,
                columns: Columns::default(),
            });
        };
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
            Ok(MarksDto {
                history: HistoryState::Ready,
                occurrences: store.occurrences(from_ms, to_ms)?,
                episodes: store.episodes(from_ms, to_ms)?,
                events: store.events(from_ms, to_ms)?,
                gaps: store.gaps(from_ms, to_ms)?,
            })
        })?;
        Ok(found.unwrap_or(MarksDto {
            history: HistoryState::Empty,
            occurrences: Vec::new(),
            episodes: Vec::new(),
            events: Vec::new(),
            gaps: Vec::new(),
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
            Ok(Some((occurrence, note)))
        })?;
        Ok(found.flatten().map(|(occurrence, capture)| {
            let window = &occurrence.window;
            let readings: Vec<Reading> = window.samples.iter().map(Reading::of_sample).collect();
            let start_ms = window.from_ms - window.from_ms.rem_euclid(SECOND_MS);
            let buckets = bucket(&readings, start_ms, window.to_ms + 1, SECOND_MS);
            OccurrenceDto {
                id,
                chart: columns(&buckets, start_ms, SECOND_MS, &[]),
                occurrence,
                capture,
            }
        }))
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
        Cause, Confidence, CoreLoad, DiskLoad, EpisodeKind, Evidence, MachineInfo, Metric,
        MinuteSample, OccurrenceKind, Sample, Severity, Suspect, Trigger, Window,
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
            .timeline(DAY_START_MS, DAY_START_MS + 6 * HOUR_MS, 600)
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
            .timeline(DAY_START_MS, DAY_START_MS + 180_000, 1_000)
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
            .timeline(DAY_START_MS, DAY_START_MS + 24 * HOUR_MS, 2_000)
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
            .timeline(DAY_START_MS, DAY_START_MS + HOUR_MS, 500)
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
            .timeline(DAY_START_MS, DAY_START_MS + 60_000, 60)
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

    #[test]
    fn step_fits_the_points_in_whole_units() {
        assert_eq!(step(HOUR_MS, SECOND_MS, 1_000), 4_000);
        assert_eq!(step(HOUR_MS, SECOND_MS, 0), HOUR_MS);
        assert_eq!(step(60_000, SECOND_MS, 1_000_000), SECOND_MS);
        assert_eq!(step(24 * HOUR_MS, MINUTE_MS, 600), 3 * MINUTE_MS);
    }
}
