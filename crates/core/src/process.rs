use std::collections::HashMap;

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

/// The latest counters of every process in `snapshots`, oldest first; one that exited keeps its last.
pub fn last_seen(snapshots: &[&[ProcessCounters]]) -> Vec<ProcessCounters> {
    let mut latest: HashMap<(u32, i64), &ProcessCounters> = HashMap::new();
    for p in snapshots.iter().flat_map(|s| s.iter()) {
        latest.insert((p.pid, p.create_time), p);
    }
    latest.into_values().cloned().collect()
}

/// Union of the `per_metric` heaviest processes by CPU, private memory, I/O and GPU, sorted by CPU.
pub fn top_processes(
    prev: &[ProcessCounters],
    curr: &[ProcessCounters],
    elapsed_ms: u64,
    logical_cpus: u32,
    per_metric: usize,
    gpu: &HashMap<u32, f32>,
) -> Vec<ProcessSample> {
    let previous: HashMap<(u32, i64), &ProcessCounters> =
        prev.iter().map(|p| ((p.pid, p.create_time), p)).collect();
    let elapsed_s = elapsed_ms as f64 / 1000.0;
    let capacity_100ns = elapsed_ms as f64 * 10_000.0 * f64::from(logical_cpus);

    let samples: Vec<ProcessSample> = curr
        .iter()
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
        .collect();

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
