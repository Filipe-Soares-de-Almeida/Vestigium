use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::process::per_program;
use crate::{Metric, ProcessTop, Sample, threshold};

/// Below the threshold this long ends an episode; a shorter dip does not split it.
pub const QUIET_MS: i64 = 10_000;
/// At most one hint of each kind in this span.
pub const HINT_EVERY_MS: i64 = 10 * 60_000;

/// A stretch of pressure on one resource, or a hint that sensitive mode would see more.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EpisodeKind {
    MemoryHigh,
    DiskBusy { disk: String },
    CpuSaturated,
    DpcIsrHint { core: u16 },
    DiskLatencyHint { disk: String },
}

impl EpisodeKind {
    /// A short sign that sensitive mode would see more, not a stretch of pressure.
    pub fn is_hint(&self) -> bool {
        matches!(
            self,
            EpisodeKind::DpcIsrHint { .. } | EpisodeKind::DiskLatencyHint { .. }
        )
    }
}

/// The process that weighed most during an episode; `value` is bytes, bytes/s or percent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Culprit {
    pub pid: u32,
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub kind: EpisodeKind,
    pub start_ms: i64,
    pub end_ms: i64,
    /// Worst value of the metric that crossed the threshold.
    pub peak: f32,
    pub culprit: Option<Culprit>,
    /// Watching stopped before the episode ended.
    pub truncated: bool,
}

/// What a culprit is judged by.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Weigh {
    MaxPrivate,
    MeanIo,
    MeanCpu,
    Nothing,
}

#[derive(Debug)]
struct Run {
    kind: EpisodeKind,
    start_ms: i64,
    last_above_ms: i64,
    peak: f32,
    weigh: Weigh,
    min_ms: i64,
    /// Per pid: name and the sum or maximum of what `weigh` looks at.
    load: HashMap<u32, (String, f64)>,
    tops: u32,
}

impl Run {
    fn qualifies(&self) -> bool {
        self.last_above_ms - self.start_ms >= self.min_ms
    }

    fn take_top(&mut self, top: &ProcessTop) {
        self.tops += 1;
        for p in &top.entries {
            let value = match self.weigh {
                Weigh::MaxPrivate => p.private_bytes as f64,
                Weigh::MeanIo => f64::from(p.read_bps) + f64::from(p.write_bps),
                Weigh::MeanCpu => f64::from(p.cpu_pct),
                Weigh::Nothing => return,
            };
            let entry = self.load.entry(p.pid).or_insert((p.name.clone(), 0.0));
            entry.1 = match self.weigh {
                Weigh::MaxPrivate => entry.1.max(value),
                _ => entry.1 + value,
            };
        }
    }

    fn into_episode(self, truncated: bool) -> Episode {
        let tops = f64::from(self.tops.max(1));
        let weigh = self.weigh;
        let mut load: Vec<(u32, String, f64)> = self
            .load
            .into_iter()
            .map(|(pid, (name, value))| (pid, name, value))
            .collect();
        // CPU counts every process of a program together, as a build runs many short ones.
        if weigh == Weigh::MeanCpu {
            load = per_program(load);
        }
        let culprit = load
            .into_iter()
            .map(|(pid, name, value)| Culprit {
                pid,
                name,
                value: if weigh == Weigh::MaxPrivate {
                    value
                } else {
                    value / tops
                },
            })
            .filter(|c| c.value > 0.0)
            .max_by(|a, b| a.value.total_cmp(&b.value).then(b.pid.cmp(&a.pid)));
        Episode {
            kind: self.kind,
            start_ms: self.start_ms,
            end_ms: self.last_above_ms,
            peak: self.peak,
            culprit,
            truncated,
        }
    }
}

/// Finds memory, disk and CPU pressure, and the DPC/ISR and disk-latency hints of the default mode.
#[derive(Debug, Default)]
pub struct EpisodeDetector {
    runs: Vec<Run>,
    /// End of the last hint emitted, per hint kind: 0 DPC/ISR, 1 disk latency.
    last_hint: [Option<i64>; 2],
}

impl EpisodeDetector {
    pub fn new() -> EpisodeDetector {
        EpisodeDetector::default()
    }

    /// The episodes this sample ends; hints are left out while sensitive mode is on.
    pub fn push_sample(&mut self, sample: &Sample, sensitive: bool) -> Vec<Episode> {
        let ts = sample.ts_ms;
        let mut above: Vec<(EpisodeKind, f32, Weigh, i64)> = Vec::new();
        if sample.mem_total_bytes > 0 {
            let used = 1.0 - sample.mem_avail_bytes as f64 / sample.mem_total_bytes as f64;
            if used > 0.85 {
                above.push((
                    EpisodeKind::MemoryHigh,
                    (used * 100.0) as f32,
                    Weigh::MaxPrivate,
                    120_000,
                ));
            }
        }
        for disk in &sample.disks {
            if disk.active_pct >= 99.0 {
                above.push((
                    EpisodeKind::DiskBusy {
                        disk: disk.name.clone(),
                    },
                    disk.active_pct,
                    Weigh::MeanIo,
                    30_000,
                ));
            }
        }
        if !sample.cores.is_empty() && sample.cores.iter().all(|c| c.cpu_pct > 95.0) {
            above.push((
                EpisodeKind::CpuSaturated,
                sample.cpu_pct,
                Weigh::MeanCpu,
                60_000,
            ));
        }
        if sensitive {
            self.runs.retain(|run| !is_hint(&run.kind));
        } else {
            for (core, load) in sample.cores.iter().enumerate() {
                let busy = load.dpc_pct + load.isr_pct;
                if busy >= threshold(Metric::CoreDpcIsrPct) {
                    above.push((
                        EpisodeKind::DpcIsrHint { core: core as u16 },
                        busy,
                        Weigh::Nothing,
                        3_000,
                    ));
                }
            }
            for disk in &sample.disks {
                if disk.latency_ms >= 100.0 && disk.active_pct >= 50.0 {
                    above.push((
                        EpisodeKind::DiskLatencyHint {
                            disk: disk.name.clone(),
                        },
                        disk.latency_ms,
                        Weigh::MeanIo,
                        3_000,
                    ));
                }
            }
        }

        for (kind, value, weigh, min_ms) in above {
            match self.runs.iter_mut().find(|run| run.kind == kind) {
                Some(run) => {
                    run.last_above_ms = ts;
                    run.peak = run.peak.max(value);
                }
                None => self.runs.push(Run {
                    kind,
                    start_ms: ts,
                    last_above_ms: ts,
                    peak: value,
                    weigh,
                    min_ms,
                    load: HashMap::new(),
                    tops: 0,
                }),
            }
        }

        let (ended, open): (Vec<Run>, Vec<Run>) = std::mem::take(&mut self.runs)
            .into_iter()
            .partition(|run| ts - run.last_above_ms >= QUIET_MS);
        self.runs = open;
        self.finish(ended, false)
    }

    /// Counts a process ranking toward the culprit of every open episode.
    pub fn push_top(&mut self, top: &ProcessTop) {
        for run in &mut self.runs {
            run.take_top(top);
        }
    }

    /// Closes every open episode, as when watching pauses or stops.
    pub fn close_all(&mut self) -> Vec<Episode> {
        let runs = std::mem::take(&mut self.runs);
        self.finish(runs, true)
    }

    /// The pressure episodes, not hints, that have lasted long enough to count and not ended.
    pub fn open_pressure(&self) -> Vec<EpisodeKind> {
        self.runs
            .iter()
            .filter(|run| !run.kind.is_hint() && run.qualifies())
            .map(|run| run.kind.clone())
            .collect()
    }

    fn finish(&mut self, runs: Vec<Run>, truncated: bool) -> Vec<Episode> {
        let mut episodes = Vec::new();
        for run in runs.into_iter().filter(Run::qualifies) {
            if let Some(slot) = hint_slot(&run.kind) {
                if self.last_hint[slot].is_some_and(|last| run.start_ms - last < HINT_EVERY_MS) {
                    continue;
                }
                self.last_hint[slot] = Some(run.last_above_ms);
            }
            episodes.push(run.into_episode(truncated));
        }
        episodes.sort_by_key(|e| e.start_ms);
        episodes
    }
}

fn is_hint(kind: &EpisodeKind) -> bool {
    hint_slot(kind).is_some()
}

fn hint_slot(kind: &EpisodeKind) -> Option<usize> {
    match kind {
        EpisodeKind::DpcIsrHint { .. } => Some(0),
        EpisodeKind::DiskLatencyHint { .. } => Some(1),
        _ => None,
    }
}
