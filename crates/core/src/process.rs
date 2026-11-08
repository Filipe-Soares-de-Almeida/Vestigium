use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

/// Cumulative counters of one process, as read from one snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessCounters {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
    /// Creation time as a Windows FILETIME; tells a reused PID apart.
    pub create_time: i64,
    pub cpu_time_100ns: u64,
    pub working_set: u64,
    pub private_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub threads: u32,
    pub context_switches: u64,
}

/// Rates of one process over the interval between two snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessSample {
    pub pid: u32,
    pub name: String,
    /// Share of the whole machine, 0 to 100.
    pub cpu_pct: f32,
    pub private_bytes: u64,
    pub read_bps: f32,
    pub write_bps: f32,
    /// Busiest GPU engine this process uses, 0 to 100.
    pub gpu_pct: f32,
}

/// The busiest processes at one moment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessTop {
    /// Unix epoch, milliseconds.
    pub ts_ms: i64,
    pub entries: Vec<ProcessSample>,
}

/// The first stored layout of `ProcessSample`, kept to read older history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessSampleV1 {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub private_bytes: u64,
    pub read_bps: f32,
    pub write_bps: f32,
}

/// The first stored layout of `ProcessTop`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessTopV1 {
    pub ts_ms: i64,
    pub entries: Vec<ProcessSampleV1>,
}

impl From<ProcessTopV1> for ProcessTop {
    fn from(old: ProcessTopV1) -> ProcessTop {
        ProcessTop {
            ts_ms: old.ts_ms,
            entries: old
                .entries
                .into_iter()
                .map(|p| ProcessSample {
                    pid: p.pid,
                    name: p.name,
                    cpu_pct: p.cpu_pct,
                    private_bytes: p.private_bytes,
                    read_bps: p.read_bps,
                    write_bps: p.write_bps,
                    gpu_pct: 0.0,
                })
                .collect(),
        }
    }
}

/// Sums `(pid, name, value)` per program name, keeping the pid of its heaviest process; heaviest first.
pub(crate) fn per_program(
    items: impl IntoIterator<Item = (u32, String, f64)>,
) -> Vec<(u32, String, f64)> {
    merge_programs(items, |value| *value, |total, value| *total += value)
}

/// Every program in `samples` once, its processes summed under the pid of its busiest by CPU;
/// busiest first.
pub fn programs(samples: &[ProcessSample]) -> Vec<ProcessSample> {
    let items = samples.iter().map(|s| (s.pid, s.name.clone(), s.clone()));
    merge_programs(
        items,
        |s| f64::from(s.cpu_pct),
        |total, s| {
            total.cpu_pct += s.cpu_pct;
            total.private_bytes = total.private_bytes.saturating_add(s.private_bytes);
            total.read_bps += s.read_bps;
            total.write_bps += s.write_bps;
            // Each process reports its busiest engine; the sum never passes a whole engine.
            total.gpu_pct = (total.gpu_pct + s.gpu_pct).min(100.0);
        },
    )
    .into_iter()
    .map(|(pid, _, total)| ProcessSample { pid, ..total })
    .collect()
}

/// Each program's use across `tops`: mean CPU, I/O and GPU and peak private memory, under the pid
/// of its busiest moment; busiest by CPU first.
pub fn usage(tops: &[ProcessTop]) -> Vec<ProcessSample> {
    let mut used: HashMap<String, (f32, ProcessSample)> = HashMap::new();
    for top in tops {
        for p in programs(&top.entries) {
            match used.entry(p.name.clone()) {
                Entry::Vacant(slot) => {
                    slot.insert((p.cpu_pct, p));
                }
                Entry::Occupied(mut slot) => {
                    let (busiest, total) = slot.get_mut();
                    if p.cpu_pct > *busiest {
                        (*busiest, total.pid) = (p.cpu_pct, p.pid);
                    }
                    total.cpu_pct += p.cpu_pct;
                    total.private_bytes = total.private_bytes.max(p.private_bytes);
                    total.read_bps += p.read_bps;
                    total.write_bps += p.write_bps;
                    total.gpu_pct += p.gpu_pct;
                }
            }
        }
    }
    let count = tops.len().max(1) as f32;
    let mut out: Vec<ProcessSample> = used
        .into_values()
        .map(|(_, p)| ProcessSample {
            cpu_pct: p.cpu_pct / count,
            read_bps: p.read_bps / count,
            write_bps: p.write_bps / count,
            gpu_pct: p.gpu_pct / count,
            ..p
        })
        .collect();
    out.sort_by(|a, b| {
        b.cpu_pct
            .total_cmp(&a.cpu_pct)
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// Merges `(pid, name, value)` per program name, keeping the pid of its heaviest by `weight`;
/// heaviest first.
fn merge_programs<T>(
    items: impl IntoIterator<Item = (u32, String, T)>,
    weight: impl Fn(&T) -> f64,
    add: impl Fn(&mut T, T),
) -> Vec<(u32, String, T)> {
    let mut programs: HashMap<String, (u32, f64, T)> = HashMap::new();
    for (pid, name, value) in items {
        let heft = weight(&value);
        match programs.entry(name) {
            Entry::Vacant(slot) => {
                slot.insert((pid, heft, value));
            }
            Entry::Occupied(mut slot) => {
                let (top_pid, top_heft, total) = slot.get_mut();
                if heft > *top_heft || (heft == *top_heft && pid < *top_pid) {
                    (*top_pid, *top_heft) = (pid, heft);
                }
                add(total, value);
            }
        }
    }
    let mut merged: Vec<(u32, String, T)> = programs
        .into_iter()
        .map(|(name, (pid, _, total))| (pid, name, total))
        .collect();
    merged.sort_by(|a, b| weight(&b.2).total_cmp(&weight(&a.2)).then(a.0.cmp(&b.0)));
    merged
}

/// The latest counters of every process in `snapshots`, oldest first; one that exited keeps its last.
pub fn last_seen(snapshots: &[&[ProcessCounters]]) -> Vec<ProcessCounters> {
    let mut latest: HashMap<(u32, i64), &ProcessCounters> = HashMap::new();
    for p in snapshots.iter().flat_map(|s| s.iter()) {
        latest.insert((p.pid, p.create_time), p);
    }
    latest.into_values().cloned().collect()
}

/// Rates of every process in `curr` over the `elapsed_ms` since `prev`; the idle process is left out.
pub fn process_rates(
    prev: &[ProcessCounters],
    curr: &[ProcessCounters],
    elapsed_ms: u64,
    logical_cpus: u32,
    gpu: &HashMap<u32, f32>,
) -> Vec<ProcessSample> {
    let previous: HashMap<(u32, i64), &ProcessCounters> =
        prev.iter().map(|p| ((p.pid, p.create_time), p)).collect();
    let elapsed_s = elapsed_ms as f64 / 1000.0;
    let capacity_100ns = elapsed_ms as f64 * 10_000.0 * f64::from(logical_cpus);

    curr.iter()
        .filter(|p| p.pid != 0)
        .map(|p| {
            let (cpu, read, write) = match previous.get(&(p.pid, p.create_time)) {
                Some(old) => (
                    p.cpu_time_100ns.saturating_sub(old.cpu_time_100ns),
                    p.read_bytes.saturating_sub(old.read_bytes),
                    p.write_bytes.saturating_sub(old.write_bytes),
                ),
                // Missing from a full earlier snapshot: it started inside the interval.
                None if !prev.is_empty() => (p.cpu_time_100ns, p.read_bytes, p.write_bytes),
                None => (0, 0, 0),
            };
            let rate = |delta: u64| {
                if elapsed_s > 0.0 {
                    (delta as f64 / elapsed_s) as f32
                } else {
                    0.0
                }
            };
            ProcessSample {
                pid: p.pid,
                name: p.name.clone(),
                cpu_pct: if capacity_100ns > 0.0 {
                    (cpu as f64 / capacity_100ns * 100.0) as f32
                } else {
                    0.0
                },
                private_bytes: p.private_bytes,
                read_bps: rate(read),
                write_bps: rate(write),
                gpu_pct: gpu.get(&p.pid).copied().unwrap_or(0.0),
            }
        })
        .collect()
}

/// One program's use at one ranking.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgramPoint {
    /// Unix epoch, milliseconds.
    pub ts_ms: i64,
    /// Share of the whole machine, 0 to 100.
    pub cpu_pct: f32,
    pub private_bytes: u64,
    pub io_bps: f32,
}

/// A program's use in one ranking, its name held as an index into the ring's names.
#[derive(Clone, Copy, Debug)]
struct ProgramUse {
    name: u32,
    cpu_pct: f32,
    private_mb: u32,
    io_kbps: u32,
}

/// Every program's use at each ranking of a recent span, kept compact in memory: names are
/// stored once and the figures in MB and KB/s.
#[derive(Debug)]
pub struct ProgramRing {
    span_ms: i64,
    names: Vec<String>,
    ids: HashMap<String, u32>,
    frames: VecDeque<(i64, Vec<ProgramUse>)>,
}

impl ProgramRing {
    pub fn new(span_ms: i64) -> ProgramRing {
        ProgramRing {
            span_ms,
            names: Vec::new(),
            ids: HashMap::new(),
            frames: VecDeque::new(),
        }
    }

    /// Adds the programs of one ranking and drops the rankings older than the span.
    pub fn push(&mut self, ts_ms: i64, programs: &[ProcessSample]) {
        while self
            .frames
            .front()
            .is_some_and(|(at, _)| *at < ts_ms - self.span_ms)
        {
            self.frames.pop_front();
        }
        if self.names.len() > 4 * programs.len().max(64) {
            self.compact();
        }
        let uses = programs
            .iter()
            .map(|p| ProgramUse {
                name: self.id(&p.name),
                cpu_pct: p.cpu_pct,
                private_mb: u32::try_from(p.private_bytes >> 20).unwrap_or(u32::MAX),
                io_kbps: ((p.read_bps + p.write_bps) / 1024.0) as u32,
            })
            .collect();
        self.frames.push_back((ts_ms, uses));
    }

    /// The rankings that list `name`, oldest first.
    pub fn history(&self, name: &str) -> Vec<ProgramPoint> {
        let Some(&id) = self.ids.get(name) else {
            return Vec::new();
        };
        self.frames
            .iter()
            .filter_map(|(ts_ms, uses)| {
                let used = uses.iter().find(|u| u.name == id)?;
                Some(ProgramPoint {
                    ts_ms: *ts_ms,
                    cpu_pct: used.cpu_pct,
                    private_bytes: u64::from(used.private_mb) << 20,
                    io_bps: used.io_kbps as f32 * 1024.0,
                })
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.names.clear();
        self.ids.clear();
    }

    fn id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = self.names.len() as u32;
        self.names.push(name.to_string());
        self.ids.insert(name.to_string(), id);
        id
    }

    /// Forgets the names no kept ranking uses any more.
    fn compact(&mut self) {
        let mut names = Vec::new();
        let mut ids = HashMap::new();
        for (_, uses) in &mut self.frames {
            for used in uses {
                let name = &self.names[used.name as usize];
                used.name = *ids.entry(name.clone()).or_insert_with(|| {
                    names.push(name.clone());
                    (names.len() - 1) as u32
                });
            }
        }
        self.names = names;
        self.ids = ids;
    }
}

/// The CPU one process used over a sliding span, from readings of its cumulative time.
#[derive(Clone, Debug)]
pub struct CpuWindow {
    span_ms: f64,
    readings: VecDeque<(f64, u64)>,
}

impl CpuWindow {
    pub fn new(span_ms: f64) -> CpuWindow {
        CpuWindow {
            span_ms,
            readings: VecDeque::new(),
        }
    }

    /// Adds a reading taken at `at_ms` on a steady clock; returns the share of the whole machine
    /// used since the oldest reading kept, 0 to 100.
    pub fn push(&mut self, at_ms: f64, cpu_time_100ns: u64, logical_cpus: u32) -> Option<f32> {
        self.readings.push_back((at_ms, cpu_time_100ns));
        while self.readings.len() > 2 && at_ms - self.readings[1].0 >= self.span_ms {
            self.readings.pop_front();
        }
        let (&(then, old), &(now, new)) = (self.readings.front()?, self.readings.back()?);
        let capacity_100ns = (now - then) * 10_000.0 * f64::from(logical_cpus);
        (capacity_100ns > 0.0)
            .then(|| (new.saturating_sub(old) as f64 / capacity_100ns * 100.0) as f32)
    }

    pub fn clear(&mut self) {
        self.readings.clear();
    }
}

/// Union of the `per_metric` heaviest samples by CPU, private memory, I/O and GPU, sorted by CPU.
pub fn heaviest(samples: Vec<ProcessSample>, per_metric: usize) -> Vec<ProcessSample> {
    let mut chosen = vec![false; samples.len()];
    mark_heaviest(&samples, &mut chosen, per_metric, |s| f64::from(s.cpu_pct));
    mark_heaviest(&samples, &mut chosen, per_metric, |s| {
        s.private_bytes as f64
    });
    mark_heaviest(&samples, &mut chosen, per_metric, |s| {
        f64::from(s.read_bps) + f64::from(s.write_bps)
    });
    mark_heaviest(&samples, &mut chosen, per_metric, |s| f64::from(s.gpu_pct));

    let mut top: Vec<ProcessSample> = samples
        .into_iter()
        .zip(chosen)
        .filter_map(|(sample, keep)| keep.then_some(sample))
        .collect();
    top.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
    top
}

/// Marks the `n` samples with the largest non-zero `key`.
fn mark_heaviest(
    samples: &[ProcessSample],
    chosen: &mut [bool],
    n: usize,
    key: impl Fn(&ProcessSample) -> f64,
) {
    let mut order: Vec<usize> = (0..samples.len())
        .filter(|&i| key(&samples[i]) > 0.0)
        .collect();
    order.sort_by(|&a, &b| key(&samples[b]).total_cmp(&key(&samples[a])));
    for &i in order.iter().take(n) {
        chosen[i] = true;
    }
}
