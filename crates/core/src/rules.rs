use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::episode::MEMORY_HIGH_PCT;
use crate::process::per_program;
use crate::{
    CrashKind, DiskLoad, EventMeaning, Media, ProcessSample, ProcessTop, RoutineKind, Sample,
    Trigger, WinEvent, Window, classify,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// A value the rules read; percentages run 0 to 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Metric {
    DiskLatencyMs,
    DiskQueue,
    DiskActivePct,
    MemAvailPct,
    HardFaultsPerS,
    CpuPct,
    CoreDpcIsrPct,
    GpuPct,
    PerfLimitPct,
    PrivateBytes,
    IoBytesPerS,
}

/// Where a rule crosses for each metric; see `crosses_down` for the direction.
pub fn threshold(metric: Metric) -> f32 {
    match metric {
        Metric::DiskLatencyMs => 200.0,
        Metric::DiskQueue => 8.0,
        Metric::DiskActivePct => 90.0,
        Metric::MemAvailPct => 5.0,
        Metric::HardFaultsPerS => 1_000.0,
        Metric::CpuPct => 95.0,
        Metric::CoreDpcIsrPct => 25.0,
        Metric::GpuPct => 95.0,
        Metric::PerfLimitPct => 95.0,
        Metric::PrivateBytes | Metric::IoBytesPerS => 0.0,
    }
}

/// Metrics that cross by falling below their threshold: free memory and the performance ceiling.
pub fn crosses_down(metric: Metric) -> bool {
    matches!(metric, Metric::MemAvailPct | Metric::PerfLimitPct)
}

/// What a suspect blames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Cause {
    Storage {
        disk: Option<String>,
    },
    DiskIo {
        disk: String,
        hdd: bool,
        process: Option<String>,
    },
    Memory {
        process: Option<String>,
    },
    Cpu {
        processes: Vec<String>,
    },
    Driver {
        core: u16,
    },
    GpuDriver,
    Throttling,
    App {
        name: String,
    },
    Unclear,
    DriverModule {
        module: String,
        intermediary: bool,
    },
    Frames {
        app: String,
    },
    BlueScreen {
        code: u32,
    },
    /// The PC stopped responding and was turned off with the power button.
    Unresponsive,
    /// The PC fell while going to sleep, hibernating or waking.
    SleepTransition,
    /// Nothing recorded the fall: a power cut, a hard hang, or a crash that could not write.
    PowerCut {
        no_dump: bool,
    },
    /// Windows logged a fatal hardware error before the fall.
    HardwareFault,
}

/// A value from the window that backs a suspect; the words come from the UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Evidence {
    Event {
        ts_ms: i64,
        provider: String,
        id: u16,
        meaning: EventMeaning,
    },
    Metric {
        ts_ms: i64,
        metric: Metric,
        value: f32,
        threshold: f32,
        subject: Option<String>,
    },
    Process {
        pid: u32,
        name: String,
        metric: Metric,
        value: f64,
    },
    Hang {
        name: String,
        duration_ms: i64,
    },
    Routine {
        module: String,
        kind: RoutineKind,
        count: u64,
        max_us: u32,
        at_least_100us: u64,
        at_least_1ms: u64,
        core_pct: f32,
    },
    LongRoutine {
        ts_ms: i64,
        module: String,
        kind: RoutineKind,
        us: u32,
    },
    Frames {
        app: String,
        frames: u32,
        hitches: u32,
        worst_ms: f32,
        median_ms: f32,
    },
    Hitch {
        ts_ms: i64,
        app: String,
        frame_ms: f32,
    },
    Hiccup {
        ts_ms: i64,
        lag_ms: f32,
    },
    CaptureLoss {
        lost_events: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NextStep {
    UpdateStorageDriver,
    CheckDiskCable,
    RunDiskTest,
    FindDiskHeavyApp,
    CloseMemoryHeavyApp,
    CheckCooling,
    CheckPowerPlan,
    UpdateGpuDriver,
    EnableSensitiveMode,
    DeepCapture,
    ReportToAppVendor,
    UpdateDriver,
    BackUpNow,
    ReplaceDisk,
    FreeDiskSpace,
    LetWindowsManagePageFile,
    CheckHardwareStability,
    KeepOneAntivirus,
    ReviewStartupApps,
    WaitForShutdown,
    MoveWindowsToSsd,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Suspect {
    pub cause: Cause,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
    pub next_steps: Vec<NextStep>,
}

/// The span the rules look at first: from the first trigger's start to its end.
#[derive(Clone, Copy, Debug)]
struct Focus {
    start: i64,
    end: i64,
}

impl Focus {
    fn of(triggers: &[Trigger], window: &Window) -> Focus {
        triggers.iter().min_by_key(|t| t.start_ms()).map_or(
            Focus {
                start: window.from_ms,
                end: window.from_ms,
            },
            |first| Focus {
                start: first.start_ms(),
                end: first.end_ms(),
            },
        )
    }
}

type Rule = fn(Focus, &Window) -> Option<Suspect>;

/// Every rule that matches, most confident first and in rule order after that; `Unclear` when none does.
pub fn suspects(triggers: &[Trigger], window: &Window) -> Vec<Suspect> {
    let focus = Focus::of(triggers, window);
    let rules: [Rule; 7] = [
        storage_reset,
        disk_latency,
        memory,
        cpu,
        dpc_isr,
        gpu_driver,
        throttling,
    ];
    let mut found: Vec<Suspect> = rules
        .iter()
        .filter_map(|rule| rule(focus, window))
        .collect();
    found.extend(app(triggers, window));
    found.extend(crash(triggers, window));
    if found.is_empty() {
        found.push(unclear(window));
    }
    found.sort_by_key(|s| std::cmp::Reverse(s.confidence));
    found
}

const STORAGE_EVENTS: &[EventMeaning] = &[
    EventMeaning::StorageReset,
    EventMeaning::DiskRetry,
    EventMeaning::DiskBadBlock,
    EventMeaning::PagingError,
];

fn storage_reset(_: Focus, window: &Window) -> Option<Suspect> {
    let mut evidence = events_meaning(window, STORAGE_EVENTS);
    if evidence.is_empty() {
        return None;
    }
    let worst = worst_disk(&window.samples, |d| d.latency_ms);
    if let Some((ts_ms, disk, value)) = &worst {
        evidence.push(metric(*ts_ms, Metric::DiskLatencyMs, *value, Some(disk)));
    }
    Some(Suspect {
        cause: Cause::Storage {
            disk: worst
                .filter(|(_, _, value)| *value > threshold(Metric::DiskLatencyMs))
                .map(|(_, disk, _)| disk),
        },
        confidence: Confidence::High,
        evidence,
        next_steps: vec![
            NextStep::UpdateStorageDriver,
            NextStep::CheckDiskCable,
            NextStep::RunDiskTest,
        ],
    })
}

fn disk_latency(focus: Focus, window: &Window) -> Option<Suspect> {
    let samples = between(&window.samples, focus.start - 10_000, focus.end + 5_000);
    let latency = worst_disk(samples.iter().copied(), |d| d.latency_ms)
        .filter(|(_, _, v)| *v > threshold(Metric::DiskLatencyMs));
    let queue = worst_disk(samples.iter().copied(), |d| d.queue_len)
        .filter(|(_, _, v)| *v >= threshold(Metric::DiskQueue));
    let disk = latency.as_ref().or(queue.as_ref())?.1.clone();
    let hdd = window
        .machine
        .disks
        .iter()
        .any(|d| d.name == disk && d.media == Media::Hdd);
    let active = samples
        .iter()
        .filter_map(|s| {
            s.disks
                .iter()
                .find(|d| d.name == disk)
                .map(|d| (s.ts_ms, d.active_pct))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));

    let mut evidence = Vec::new();
    for (ts_ms, name, value) in latency.iter() {
        evidence.push(metric(*ts_ms, Metric::DiskLatencyMs, *value, Some(name)));
    }
    for (ts_ms, name, value) in queue.iter() {
        evidence.push(metric(*ts_ms, Metric::DiskQueue, *value, Some(name)));
    }
    if let Some((ts_ms, value)) = active {
        evidence.push(metric(ts_ms, Metric::DiskActivePct, value, Some(&disk)));
    }
    let tops = tops_or_all(window, focus.start - 10_000, focus.end + 5_000);
    let busiest = ranked(&tops, Metric::IoBytesPerS, Summary::Mean)
        .into_iter()
        .next();
    if let Some(p) = &busiest {
        evidence.push(p.clone());
    }
    let heavy = hdd && active.is_some_and(|(_, v)| v >= threshold(Metric::DiskActivePct));
    let mut next_steps = vec![NextStep::FindDiskHeavyApp];
    if hdd {
        next_steps.push(NextStep::RunDiskTest);
    }
    Some(Suspect {
        cause: Cause::DiskIo {
            disk,
            hdd,
            process: busiest.and_then(process_name),
        },
        confidence: if heavy {
            Confidence::High
        } else {
            Confidence::Medium
        },
        evidence,
        next_steps,
    })
}

fn memory(focus: Focus, window: &Window) -> Option<Suspect> {
    let samples = between(&window.samples, focus.start - 30_000, focus.start);
    let total = window.machine.mem_total_bytes;
    let avail = samples
        .iter()
        .filter_map(|s| avail_pct(s, total).map(|pct| (s.ts_ms, pct)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, pct)| *pct < threshold(Metric::MemAvailPct));
    // With memory to spare, hard faults are files and programs being read in, not pages pushed out.
    let faults = samples
        .iter()
        .filter(|s| avail_pct(s, total).is_some_and(|pct| pct < 100.0 - MEMORY_HIGH_PCT))
        .map(|s| (s.ts_ms, s.hard_faults_per_s))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, v)| *v >= threshold(Metric::HardFaultsPerS));
    if avail.is_none() && faults.is_none() {
        return None;
    }
    let mut evidence = Vec::new();
    if let Some((ts_ms, value)) = avail {
        evidence.push(metric(ts_ms, Metric::MemAvailPct, value, None));
    }
    if let Some((ts_ms, value)) = faults {
        evidence.push(metric(ts_ms, Metric::HardFaultsPerS, value, None));
    }
    let all: Vec<&ProcessTop> = window.tops.iter().collect();
    let biggest = ranked(&all, Metric::PrivateBytes, Summary::Max)
        .into_iter()
        .next();
    if let Some(p) = &biggest {
        evidence.push(p.clone());
    }
    Some(Suspect {
        cause: Cause::Memory {
            process: biggest.and_then(process_name),
        },
        confidence: Confidence::Medium,
        evidence,
        next_steps: vec![NextStep::CloseMemoryHeavyApp],
    })
}

fn cpu(focus: Focus, window: &Window) -> Option<Suspect> {
    let from = focus.start - 10_000;
    let to = focus.end + 5_000;
    let mut saturated_ms = 0;
    let mut peak: Option<(i64, f32)> = None;
    for (i, s) in window.samples.iter().enumerate() {
        if s.ts_ms < from || s.ts_ms > to {
            continue;
        }
        // A late sample covers everything since the one before it, as the CPU counters do.
        let since = i
            .checked_sub(1)
            .map_or(s.ts_ms - 1_000, |p| window.samples[p].ts_ms);
        if !saturated(s) && !busy_through_freeze(s, since, focus) {
            continue;
        }
        saturated_ms += s.ts_ms - since;
        if peak.is_none_or(|(_, v)| s.cpu_pct > v) {
            peak = Some((s.ts_ms, s.cpu_pct));
        }
    }
    if saturated_ms < 3_000 {
        return None;
    }
    let mut evidence = Vec::new();
    if let Some((ts_ms, value)) = peak {
        evidence.push(metric(ts_ms, Metric::CpuPct, value, None));
    }
    let tops = tops_or_all(window, from, to);
    let per_process = ranked(&tops, Metric::CpuPct, Summary::Mean)
        .into_iter()
        .filter_map(|e| match e {
            Evidence::Process {
                pid, name, value, ..
            } => Some((pid, name, value)),
            _ => None,
        });
    let heaviest: Vec<Evidence> = per_program(per_process)
        .into_iter()
        .take(3)
        .map(|(pid, name, value)| Evidence::Process {
            pid,
            name,
            metric: Metric::CpuPct,
            value,
        })
        .collect();
    let processes = heaviest
        .iter()
        .filter_map(|p| process_name(p.clone()))
        .collect();
    evidence.extend(heaviest);
    Some(Suspect {
        cause: Cause::Cpu { processes },
        confidence: Confidence::Medium,
        evidence,
        next_steps: Vec::new(),
    })
}

/// Whether every core of a sample spanning the freeze was busy for 95% of the frozen time,
/// so the part of its span before the freeze does not hide a saturated CPU.
fn busy_through_freeze(sample: &Sample, since_ms: i64, focus: Focus) -> bool {
    let frozen_ms = sample.ts_ms.min(focus.end) - since_ms.max(focus.start);
    let span_ms = sample.ts_ms - since_ms;
    let needed_ms = threshold(Metric::CpuPct) / 100.0 * frozen_ms as f32;
    frozen_ms > 0
        && !sample.cores.is_empty()
        && sample
            .cores
            .iter()
            .all(|c| c.cpu_pct / 100.0 * span_ms as f32 >= needed_ms)
}

fn dpc_isr(focus: Focus, window: &Window) -> Option<Suspect> {
    let (ts_ms, core, value) = between(&window.samples, focus.start - 5_000, focus.end + 5_000)
        .iter()
        .flat_map(|s| {
            s.cores
                .iter()
                .enumerate()
                .map(|(core, c)| (s.ts_ms, core as u16, c.dpc_pct + c.isr_pct))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .filter(|(_, _, v)| *v >= threshold(Metric::CoreDpcIsrPct))?;
    Some(Suspect {
        cause: Cause::Driver { core },
        confidence: if value >= 40.0 {
            Confidence::Medium
        } else {
            Confidence::Low
        },
        evidence: vec![metric(
            ts_ms,
            Metric::CoreDpcIsrPct,
            value,
            Some(&core.to_string()),
        )],
        next_steps: vec![NextStep::DeepCapture],
    })
}

fn gpu_driver(_: Focus, window: &Window) -> Option<Suspect> {
    let mut evidence = events_meaning(window, &[EventMeaning::DisplayReset]);
    if evidence.is_empty() {
        return None;
    }
    let gpu = window
        .samples
        .iter()
        .filter_map(|s| s.gpu_pct.map(|v| (s.ts_ms, v)))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((ts_ms, value)) = gpu {
        evidence.push(metric(ts_ms, Metric::GpuPct, value, None));
    }
    Some(Suspect {
        cause: Cause::GpuDriver,
        confidence: Confidence::High,
        evidence,
        next_steps: vec![NextStep::UpdateGpuDriver],
    })
}

fn throttling(focus: Focus, window: &Window) -> Option<Suspect> {
    let limit = between(&window.samples, focus.start - 30_000, focus.start)
        .iter()
        .map(|s| (s.ts_ms, s.perf_limit_pct))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, v)| *v < threshold(Metric::PerfLimitPct));
    let mut evidence = events_meaning(window, &[EventMeaning::CpuFirmwareLimit]);
    let reported = !evidence.is_empty();
    if let Some((ts_ms, value)) = limit {
        evidence.push(metric(ts_ms, Metric::PerfLimitPct, value, None));
    }
    if evidence.is_empty() {
        return None;
    }
    Some(Suspect {
        cause: Cause::Throttling,
        confidence: if reported {
            Confidence::Medium
        } else {
            Confidence::Low
        },
        evidence,
        next_steps: vec![NextStep::CheckCooling, NextStep::CheckPowerPlan],
    })
}

fn app(triggers: &[Trigger], window: &Window) -> Vec<Suspect> {
    if triggers.iter().any(|t| matches!(t, Trigger::Freeze { .. })) {
        return Vec::new();
    }
    let mut named: Vec<Suspect> = Vec::new();
    for trigger in triggers {
        let Trigger::AppHang {
            pid,
            name,
            start_ms,
            end_ms,
            ..
        } = trigger
        else {
            continue;
        };
        if named
            .iter()
            .any(|s| matches!(&s.cause, Cause::App { name: n } if n == name))
        {
            continue;
        }
        let mut evidence = vec![Evidence::Hang {
            name: name.clone(),
            duration_ms: end_ms - start_ms,
        }];
        for m in [Metric::CpuPct, Metric::IoBytesPerS] {
            let own = window
                .tops
                .iter()
                .flat_map(|t| &t.entries)
                .filter(|p| p.pid == *pid)
                .map(|p| process_value(p, m))
                .max_by(f64::total_cmp);
            if let Some(value) = own {
                evidence.push(Evidence::Process {
                    pid: *pid,
                    name: name.clone(),
                    metric: m,
                    value,
                });
            }
        }
        named.push(Suspect {
            cause: Cause::App { name: name.clone() },
            confidence: Confidence::High,
            evidence,
            next_steps: vec![NextStep::ReportToAppVendor],
        });
    }
    named
}

/// What Kernel-Power 41 and the events of the next boot say about a fall.
fn crash(triggers: &[Trigger], window: &Window) -> Vec<Suspect> {
    let Some(kind) = triggers.iter().find_map(|t| match t {
        Trigger::Crash { kind, .. } => Some(*kind),
        _ => None,
    }) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let fatal: Vec<Evidence> = events_meaning(window, &[EventMeaning::HardwareError])
        .into_iter()
        .filter(|e| matches!(e, Evidence::Event { id: 18, .. }))
        .collect();
    if !fatal.is_empty() {
        found.push(Suspect {
            cause: Cause::HardwareFault,
            confidence: Confidence::High,
            evidence: fatal,
            next_steps: vec![NextStep::CheckHardwareStability],
        });
    }
    let no_dump = events_meaning(window, &[EventMeaning::NoCrashDump]);
    let (cause, confidence, next_steps) = match kind {
        CrashKind::BlueScreen { code, .. } => (
            Cause::BlueScreen { code },
            Confidence::High,
            vec![NextStep::UpdateDriver],
        ),
        CrashKind::PowerButton => (
            Cause::Unresponsive,
            Confidence::Medium,
            vec![NextStep::EnableSensitiveMode],
        ),
        CrashKind::WhileSleeping => (
            Cause::SleepTransition,
            Confidence::Medium,
            vec![NextStep::UpdateDriver],
        ),
        CrashKind::NoTrace => (
            Cause::PowerCut {
                no_dump: !no_dump.is_empty(),
            },
            Confidence::Low,
            vec![NextStep::CheckHardwareStability],
        ),
        CrashKind::WhileShuttingDown => return found,
    };
    let mut evidence = events_meaning(
        window,
        &[
            EventMeaning::PowerLoss,
            EventMeaning::UnexpectedShutdown,
            EventMeaning::BugCheck,
        ],
    );
    evidence.extend(no_dump);
    found.push(Suspect {
        cause,
        confidence,
        evidence,
        next_steps,
    });
    found
}

/// The ordinary signals, to show that nothing crossed.
fn unclear(window: &Window) -> Suspect {
    let peak = |value: fn(&Sample) -> f32| {
        window
            .samples
            .iter()
            .map(|s| (s.ts_ms, value(s)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let mut evidence = Vec::new();
    if let Some((ts_ms, value)) = peak(|s| s.cpu_pct) {
        evidence.push(metric(ts_ms, Metric::CpuPct, value, None));
    }
    let total = window.machine.mem_total_bytes;
    if let Some((ts_ms, value)) = window
        .samples
        .iter()
        .filter_map(|s| avail_pct(s, total).map(|pct| (s.ts_ms, pct)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
    {
        evidence.push(metric(ts_ms, Metric::MemAvailPct, value, None));
    }
    if let Some((ts_ms, disk, value)) = worst_disk(&window.samples, |d| d.latency_ms) {
        evidence.push(metric(ts_ms, Metric::DiskLatencyMs, value, Some(&disk)));
    }
    Suspect {
        cause: Cause::Unclear,
        confidence: Confidence::Low,
        evidence,
        next_steps: vec![NextStep::EnableSensitiveMode, NextStep::DeepCapture],
    }
}

fn metric(ts_ms: i64, metric: Metric, value: f32, subject: Option<&str>) -> Evidence {
    Evidence::Metric {
        ts_ms,
        metric,
        value,
        threshold: threshold(metric),
        subject: subject.map(str::to_string),
    }
}

/// Curated events of these kinds, judged again by provider and id rather than by the stored meaning.
fn events_meaning(window: &Window, meanings: &[EventMeaning]) -> Vec<Evidence> {
    window
        .events
        .iter()
        .filter_map(|e| {
            curated_meaning(e)
                .filter(|m| meanings.contains(m))
                .map(|m| (e, m))
        })
        .map(|(e, meaning)| Evidence::Event {
            ts_ms: e.ts_ms,
            provider: e.provider.clone(),
            id: e.id,
            meaning,
        })
        .collect()
}

fn curated_meaning(event: &WinEvent) -> Option<EventMeaning> {
    classify(event.channel, &event.provider, event.id)
}

fn between(samples: &[Sample], from_ms: i64, to_ms: i64) -> Vec<&Sample> {
    samples
        .iter()
        .filter(|s| (from_ms..=to_ms).contains(&s.ts_ms))
        .collect()
}

/// The highest disk value across the samples, with when and on which disk.
pub(crate) fn worst_disk<'a>(
    samples: impl IntoIterator<Item = &'a Sample>,
    value: fn(&DiskLoad) -> f32,
) -> Option<(i64, String, f32)> {
    samples
        .into_iter()
        .flat_map(|s| s.disks.iter().map(move |d| (s.ts_ms, d)))
        .max_by(|a, b| value(a.1).total_cmp(&value(b.1)))
        .map(|(ts_ms, d)| (ts_ms, d.name.clone(), value(d)))
}

pub(crate) fn avail_pct(sample: &Sample, machine_total: u64) -> Option<f32> {
    let total = if sample.mem_total_bytes > 0 {
        sample.mem_total_bytes
    } else {
        machine_total
    };
    (total > 0).then(|| (sample.mem_avail_bytes as f64 / total as f64 * 100.0) as f32)
}

pub(crate) fn saturated(sample: &Sample) -> bool {
    !sample.cores.is_empty()
        && sample
            .cores
            .iter()
            .all(|c| c.cpu_pct >= threshold(Metric::CpuPct))
}

fn tops_or_all(window: &Window, from_ms: i64, to_ms: i64) -> Vec<&ProcessTop> {
    let inside: Vec<&ProcessTop> = window
        .tops
        .iter()
        .filter(|t| (from_ms..=to_ms).contains(&t.ts_ms))
        .collect();
    if inside.is_empty() {
        window.tops.iter().collect()
    } else {
        inside
    }
}

pub(crate) fn process_value(p: &ProcessSample, metric: Metric) -> f64 {
    match metric {
        Metric::CpuPct => f64::from(p.cpu_pct),
        Metric::PrivateBytes => p.private_bytes as f64,
        Metric::IoBytesPerS => f64::from(p.read_bps) + f64::from(p.write_bps),
        Metric::GpuPct => f64::from(p.gpu_pct),
        _ => 0.0,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Summary {
    Mean,
    Max,
}

/// Processes by the mean or the peak of `metric` across the rankings, heaviest first.
fn ranked(tops: &[&ProcessTop], metric: Metric, summary: Summary) -> Vec<Evidence> {
    let mut load: HashMap<u32, (String, f64)> = HashMap::new();
    for p in tops.iter().flat_map(|t| &t.entries) {
        let value = process_value(p, metric);
        let entry = load.entry(p.pid).or_insert((p.name.clone(), 0.0));
        entry.1 = match summary {
            Summary::Mean => entry.1 + value,
            Summary::Max => entry.1.max(value),
        };
    }
    let count = tops.len().max(1) as f64;
    let mut ranked: Vec<(u32, String, f64)> = load
        .into_iter()
        .map(|(pid, (name, value))| match summary {
            Summary::Mean => (pid, name, value / count),
            Summary::Max => (pid, name, value),
        })
        .filter(|(_, _, value)| *value > 0.0)
        .collect();
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .map(|(pid, name, value)| Evidence::Process {
            pid,
            name,
            metric,
            value,
        })
        .collect()
}

fn process_name(evidence: Evidence) -> Option<String> {
    match evidence {
        Evidence::Process { name, .. } => Some(name),
        _ => None,
    }
}
