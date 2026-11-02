use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use crate::{Cause, Confidence, Evidence, NextStep, Suspect};

/// Version of the capture file and row format.
pub const CAPTURE_VERSION: u8 = 1;
/// Lower edges of the duration buckets, in microseconds, as `tracerpt` reports them.
pub const BUCKET_BOUNDS_US: [u32; 12] = [1, 2, 3, 4, 5, 10, 25, 50, 100, 250, 500, 1000];
/// One bucket below each bound plus the last one, at or above the final bound.
pub const BUCKETS: usize = 13;
/// Individual DPC and ISR routines kept, longest first.
pub const LONGEST_KEPT: usize = 50;
/// A frame over this many times the app's median...
pub const HITCH_FACTOR: f32 = 2.5;
/// ...and at least this long, in milliseconds, is a hitch.
pub const HITCH_MIN_MS: f32 = 20.0;
pub const HITCHES_KEPT: usize = 200;
/// Share of frames over the hitch line past which an app draws only when something changes.
pub const ON_DEMAND_SHARE: f32 = 0.05;

/// Frames an app needs in a capture to be reported.
const MIN_FRAMES: usize = 60;
/// Name given to routines whose address is in no loaded image.
const UNKNOWN_MODULE: &str = "<unknown>";
/// A routine this long, in microseconds, is felt on its own.
const LONG_US: u32 = 1_000;
/// A routine this long, in microseconds, counts toward the rate of long ones.
const SLOW_US: u32 = 100;
/// Routines of at least `SLOW_US` per minute that make a driver a suspect.
const SLOW_PER_MIN: f64 = 60.0;
/// Share of one core, in percent, that makes a driver a suspect.
const CORE_PCT: f32 = 2.0;
/// Hitches in a capture that make an app a suspect.
const HITCHES_TO_BLAME: u32 = 3;
/// How close, in milliseconds, a hiccup or hitch must be to back a routine or a hitch.
const NEAR_MS: i64 = 50;
/// Items of one kind kept as evidence of a suspect.
const EVIDENCE_KEPT: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoutineKind {
    Dpc,
    Isr,
}

/// Durations of one kind of routine: count, total, longest and a histogram.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutineStats {
    pub count: u64,
    pub total_us: u64,
    pub max_us: u32,
    pub buckets: [u32; BUCKETS],
}

impl RoutineStats {
    pub fn add(&mut self, us: u32) {
        self.count += 1;
        self.total_us += u64::from(us);
        self.max_us = self.max_us.max(us);
        let bucket = BUCKET_BOUNDS_US.partition_point(|bound| *bound <= us);
        self.buckets[bucket] = self.buckets[bucket].saturating_add(1);
    }

    pub fn merge(&mut self, other: &RoutineStats) {
        self.count += other.count;
        self.total_us += other.total_us;
        self.max_us = self.max_us.max(other.max_us);
        for (mine, theirs) in self.buckets.iter_mut().zip(other.buckets) {
            *mine = mine.saturating_add(theirs);
        }
    }

    /// Routines in buckets that start at or above `us`; exact when `us` is a bucket bound.
    pub fn at_least(&self, us: u32) -> u64 {
        self.buckets
            .iter()
            .enumerate()
            .filter(|(i, _)| i.checked_sub(1).map_or(0, |j| BUCKET_BOUNDS_US[j]) >= us)
            .map(|(_, n)| u64::from(*n))
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverLatency {
    pub module: String,
    pub dpc: RoutineStats,
    pub isr: RoutineStats,
}

impl DriverLatency {
    fn total_us(&self) -> u64 {
        self.dpc.total_us + self.isr.total_us
    }

    fn max_us(&self) -> u32 {
        self.dpc.max_us.max(self.isr.max_us)
    }
}

/// One DPC or ISR that ran long.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LongRoutine {
    pub ts_ms: i64,
    pub core: u16,
    pub module: String,
    pub kind: RoutineKind,
    pub us: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppFrames {
    pub name: String,
    pub pid: u32,
    pub frames: u32,
    pub median_ms: f32,
    pub p99_ms: f32,
    pub worst_ms: f32,
    pub hitches: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameHitch {
    pub ts_ms: i64,
    pub app: String,
    pub pid: u32,
    pub frame_ms: f32,
    pub median_ms: f32,
}

/// A late wake of the stall probe during the capture.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeHiccup {
    pub ts_ms: i64,
    pub lag_ms: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureSource {
    Ok,
    Unavailable(String),
}

/// What one deep capture measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeepCapture {
    pub start_ms: i64,
    pub end_ms: i64,
    pub kernel: CaptureSource,
    pub frames: CaptureSource,
    pub lost_events: u32,
    pub drivers: Vec<DriverLatency>,
    pub longest: Vec<LongRoutine>,
    pub apps: Vec<AppFrames>,
    pub hitches: Vec<FrameHitch>,
    pub hiccups: Vec<ProbeHiccup>,
}

/// Loaded kernel images, from the session's image events.
#[derive(Clone, Debug, Default)]
pub struct ModuleMap {
    /// (base, size, name), sorted by base.
    images: Vec<(u64, u64, String)>,
}

impl ModuleMap {
    pub fn new() -> ModuleMap {
        ModuleMap::default()
    }

    /// Records an image; the name is the file name of `path`, and a second image at the same base replaces the first.
    pub fn insert(&mut self, base: u64, size: u64, path: &str) {
        let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
        let at = self.images.partition_point(|(b, _, _)| *b < base);
        if self.images.get(at).is_some_and(|(b, _, _)| *b == base) {
            self.images[at] = (base, size, name);
        } else {
            self.images.insert(at, (base, size, name));
        }
    }

    pub fn find(&self, addr: u64) -> Option<&str> {
        let at = self.images.partition_point(|(base, _, _)| *base <= addr);
        let (base, size, name) = self.images.get(at.checked_sub(1)?)?;
        (addr - base < *size).then_some(name.as_str())
    }
}

/// One of the longest routines seen, ordered so the shortest is the first to go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Kept {
    us: u32,
    ts_ms: i64,
    routine: u64,
    kind: RoutineKind,
    core: u16,
}

/// DPC and ISR durations by routine address, folded into modules at the end.
#[derive(Debug, Default)]
pub struct RoutineAggregator {
    stats: HashMap<(u64, RoutineKind), RoutineStats>,
    longest: BinaryHeap<Reverse<Kept>>,
}

impl RoutineAggregator {
    pub fn new() -> RoutineAggregator {
        RoutineAggregator::default()
    }

    pub fn add(&mut self, routine: u64, kind: RoutineKind, us: u32, core: u16, ts_ms: i64) {
        self.stats.entry((routine, kind)).or_default().add(us);
        let kept = Kept {
            us,
            ts_ms,
            routine,
            kind,
            core,
        };
        if self.longest.len() < LONGEST_KEPT {
            self.longest.push(Reverse(kept));
        } else if self.longest.peek().is_some_and(|Reverse(min)| *min < kept) {
            self.longest.pop();
            self.longest.push(Reverse(kept));
        }
    }

    /// Modules by DPC plus ISR time, most first, and the longest routines, longest first.
    pub fn finish(self, modules: &ModuleMap) -> (Vec<DriverLatency>, Vec<LongRoutine>) {
        let name = |addr: u64| modules.find(addr).unwrap_or(UNKNOWN_MODULE).to_string();
        let mut by_module: HashMap<String, DriverLatency> = HashMap::new();
        for ((routine, kind), stats) in &self.stats {
            let module = name(*routine);
            let driver = by_module
                .entry(module.clone())
                .or_insert_with(|| DriverLatency {
                    module,
                    dpc: RoutineStats::default(),
                    isr: RoutineStats::default(),
                });
            match kind {
                RoutineKind::Dpc => driver.dpc.merge(stats),
                RoutineKind::Isr => driver.isr.merge(stats),
            }
        }
        let mut drivers: Vec<DriverLatency> = by_module.into_values().collect();
        drivers.sort_by(|a, b| {
            b.total_us()
                .cmp(&a.total_us())
                .then(a.module.cmp(&b.module))
        });
        let mut kept: Vec<Kept> = self.longest.into_iter().map(|Reverse(k)| k).collect();
        kept.sort_by(|a, b| b.us.cmp(&a.us).then(a.ts_ms.cmp(&b.ts_ms)));
        let longest = kept
            .into_iter()
            .map(|k| LongRoutine {
                ts_ms: k.ts_ms,
                core: k.core,
                module: name(k.routine),
                kind: k.kind,
                us: k.us,
            })
            .collect();
        (drivers, longest)
    }
}

/// Frame times per (app, pid); hitches by `HITCH_FACTOR` times the median and `HITCH_MIN_MS`.
#[derive(Debug, Default)]
pub struct FrameAggregator {
    frames: HashMap<(String, u32), Vec<(i64, f32)>>,
}

impl FrameAggregator {
    pub fn new() -> FrameAggregator {
        FrameAggregator::default()
    }

    pub fn add(&mut self, app: &str, pid: u32, ts_ms: i64, frame_ms: f32) {
        self.frames
            .entry((app.to_string(), pid))
            .or_default()
            .push((ts_ms, frame_ms));
    }

    /// Apps with at least 60 frames, most hitches first then most frames, and the worst hitches in time order.
    pub fn finish(self) -> (Vec<AppFrames>, Vec<FrameHitch>) {
        let mut apps = Vec::new();
        let mut hitches = Vec::new();
        for ((name, pid), frames) in self.frames {
            if frames.len() < MIN_FRAMES {
                continue;
            }
            let mut sorted: Vec<f32> = frames.iter().map(|(_, ms)| *ms).collect();
            sorted.sort_by(f32::total_cmp);
            let median_ms = sorted[sorted.len() / 2];
            let p99_rank = (sorted.len() * 99).div_ceil(100);
            let mut count = 0;
            for (ts_ms, frame_ms) in &frames {
                if *frame_ms > HITCH_FACTOR * median_ms && *frame_ms >= HITCH_MIN_MS {
                    count += 1;
                    hitches.push(FrameHitch {
                        ts_ms: *ts_ms,
                        app: name.clone(),
                        pid,
                        frame_ms: *frame_ms,
                        median_ms,
                    });
                }
            }
            let app = AppFrames {
                name,
                pid,
                frames: frames.len() as u32,
                median_ms,
                p99_ms: sorted[p99_rank - 1],
                worst_ms: sorted[sorted.len() - 1],
                hitches: count,
            };
            if !renders_steadily(&app) {
                hitches.retain(|h| h.app != app.name || h.pid != app.pid);
                continue;
            }
            apps.push(app);
        }
        apps.sort_by(|a, b| {
            b.hitches
                .cmp(&a.hitches)
                .then(b.frames.cmp(&a.frames))
                .then(a.name.cmp(&b.name))
                .then(a.pid.cmp(&b.pid))
        });
        hitches.sort_by(|a, b| {
            b.frame_ms
                .total_cmp(&a.frame_ms)
                .then(a.ts_ms.cmp(&b.ts_ms))
        });
        hitches.truncate(HITCHES_KEPT);
        hitches.sort_by_key(|h| h.ts_ms);
        (apps, hitches)
    }
}

/// The desktop compositor, whose frames are the screen's, not an app's.
pub fn is_compositor(app: &str) -> bool {
    app.eq_ignore_ascii_case("dwm.exe")
}

/// An app drawing at a steady pace: not the compositor, and no more than `ON_DEMAND_SHARE` of its
/// frames over the hitch line; one that draws only when something changes has idle gaps, not hitches.
pub fn renders_steadily(app: &AppFrames) -> bool {
    !is_compositor(&app.name) && app.hitches as f32 <= ON_DEMAND_SHARE * app.frames as f32
}

/// The capture with only the apps that draw steadily and their hitches.
pub fn steady_only(capture: &DeepCapture) -> DeepCapture {
    let mut kept = capture.clone();
    kept.apps.retain(renders_steadily);
    kept.hitches.retain(|h| {
        !is_compositor(&h.app)
            && !capture
                .apps
                .iter()
                .any(|a| a.name == h.app && a.pid == h.pid && !renders_steadily(a))
    });
    kept
}

/// Frameworks that run other drivers' code, so the time they show is not theirs.
pub fn is_intermediary(module: &str) -> bool {
    const NAMES: [&str; 5] = [
        "wdf01000.sys",
        "ndis.sys",
        "storport.sys",
        "ataport.sys",
        "dxgkrnl.sys",
    ];
    NAMES.iter().any(|n| module.eq_ignore_ascii_case(n))
}

/// The kernel and the HAL: they run everyone's routines and are never named.
pub fn is_kernel_core(module: &str) -> bool {
    ["ntoskrnl.exe", "hal.dll"]
        .iter()
        .any(|n| module.eq_ignore_ascii_case(n))
}

/// A module worth naming: not the kernel core and not an address no image holds.
fn nameable(module: &str) -> bool {
    !is_kernel_core(module) && module != UNKNOWN_MODULE
}

/// Drivers and apps the whole capture names, for its own page.
pub fn capture_findings(capture: &DeepCapture) -> Vec<Suspect> {
    let capture = &steady_only(capture);
    let duration_ms = (capture.end_ms - capture.start_ms).max(1);
    let mut found: Vec<(Suspect, f64)> = capture
        .drivers
        .iter()
        .filter(|d| nameable(&d.module))
        .filter_map(|d| driver_finding(d, duration_ms))
        .collect();
    found.extend(
        capture
            .apps
            .iter()
            .filter(|a| a.hitches >= HITCHES_TO_BLAME)
            .map(|a| app_finding(a, capture)),
    );
    ranked(found, capture.lost_events)
}

fn driver_finding(driver: &DriverLatency, duration_ms: i64) -> Option<(Suspect, f64)> {
    let intermediary = is_intermediary(&driver.module);
    let minutes = duration_ms as f64 / 60_000.0;
    let core_pct = (driver.total_us() as f64 / (duration_ms as f64 * 1_000.0) * 100.0) as f32;
    let slow = driver.dpc.at_least(SLOW_US) + driver.isr.at_least(SLOW_US);
    let max_us = driver.max_us();
    let confidence = if max_us >= LONG_US {
        if intermediary {
            Confidence::Medium
        } else {
            Confidence::High
        }
    } else if slow as f64 / minutes >= SLOW_PER_MIN || core_pct >= CORE_PCT {
        if intermediary {
            Confidence::Low
        } else {
            Confidence::Medium
        }
    } else {
        return None;
    };
    let kind = if driver.isr.total_us > driver.dpc.total_us {
        RoutineKind::Isr
    } else {
        RoutineKind::Dpc
    };
    let suspect = Suspect {
        cause: Cause::DriverModule {
            module: driver.module.clone(),
            intermediary,
        },
        confidence,
        evidence: vec![Evidence::Routine {
            module: driver.module.clone(),
            kind,
            count: driver.dpc.count + driver.isr.count,
            max_us,
            at_least_100us: slow,
            at_least_1ms: driver.dpc.at_least(LONG_US) + driver.isr.at_least(LONG_US),
            core_pct,
        }],
        next_steps: update_driver(intermediary),
    };
    Some((suspect, f64::from(max_us)))
}

fn app_finding(app: &AppFrames, capture: &DeepCapture) -> (Suspect, f64) {
    let mut own: Vec<&FrameHitch> = capture
        .hitches
        .iter()
        .filter(|h| h.app == app.name && h.pid == app.pid)
        .collect();
    own.sort_by(|a, b| b.frame_ms.total_cmp(&a.frame_ms));
    let mut evidence = vec![frames_evidence(app)];
    evidence.extend(own.into_iter().take(EVIDENCE_KEPT).map(hitch_evidence));
    let suspect = Suspect {
        cause: Cause::Frames {
            app: app.name.clone(),
        },
        confidence: Confidence::Medium,
        evidence,
        next_steps: Vec::new(),
    };
    (suspect, f64::from(app.hitches))
}

/// What the capture adds to an occurrence whose window is `from_ms` to `to_ms`.
pub fn capture_suspects(from_ms: i64, to_ms: i64, capture: &DeepCapture) -> Vec<Suspect> {
    let capture = &steady_only(capture);
    let inside = |ts_ms: i64| (from_ms..=to_ms).contains(&ts_ms);
    let hiccups: Vec<&ProbeHiccup> = capture.hiccups.iter().filter(|h| inside(h.ts_ms)).collect();
    let hitches: Vec<&FrameHitch> = capture.hitches.iter().filter(|h| inside(h.ts_ms)).collect();

    let mut by_module: BTreeMap<&str, Vec<&LongRoutine>> = BTreeMap::new();
    for long in &capture.longest {
        if inside(long.ts_ms) && long.us >= LONG_US && nameable(&long.module) {
            by_module.entry(&long.module).or_default().push(long);
        }
    }
    let mut found: Vec<(Suspect, f64)> = by_module
        .into_iter()
        .map(|(module, longs)| window_driver(module, longs, &hiccups, &hitches))
        .collect();

    let mut by_app: BTreeMap<&str, Vec<&FrameHitch>> = BTreeMap::new();
    for hitch in hitches {
        by_app.entry(&hitch.app).or_default().push(hitch);
    }
    found.extend(
        by_app
            .into_iter()
            .map(|(app, own)| window_app(app, own, &hiccups, capture)),
    );
    ranked(found, capture.lost_events)
}

fn window_driver(
    module: &str,
    mut longs: Vec<&LongRoutine>,
    hiccups: &[&ProbeHiccup],
    hitches: &[&FrameHitch],
) -> (Suspect, f64) {
    longs.sort_by(|a, b| b.us.cmp(&a.us).then(a.ts_ms.cmp(&b.ts_ms)));
    let near_long = |ts_ms: i64| longs.iter().any(|l| (l.ts_ms - ts_ms).abs() <= NEAR_MS);
    let backing_hiccups: Vec<&ProbeHiccup> = hiccups
        .iter()
        .copied()
        .filter(|h| near_long(h.ts_ms))
        .collect();
    let backing_hitches: Vec<&FrameHitch> = hitches
        .iter()
        .copied()
        .filter(|h| near_long(h.ts_ms))
        .collect();
    let intermediary = is_intermediary(module);
    let backed = !backing_hiccups.is_empty() || !backing_hitches.is_empty();
    let top = longs[0].us;
    let mut evidence: Vec<Evidence> = longs
        .iter()
        .take(EVIDENCE_KEPT)
        .map(|l| Evidence::LongRoutine {
            ts_ms: l.ts_ms,
            module: l.module.clone(),
            kind: l.kind,
            us: l.us,
        })
        .collect();
    evidence.extend(
        backing_hiccups
            .into_iter()
            .take(EVIDENCE_KEPT)
            .map(hiccup_evidence),
    );
    evidence.extend(
        backing_hitches
            .into_iter()
            .take(EVIDENCE_KEPT)
            .map(hitch_evidence),
    );
    let suspect = Suspect {
        cause: Cause::DriverModule {
            module: module.to_string(),
            intermediary,
        },
        confidence: if intermediary && !backed {
            Confidence::Medium
        } else {
            Confidence::High
        },
        evidence,
        next_steps: update_driver(intermediary),
    };
    (suspect, f64::from(top))
}

fn window_app(
    app: &str,
    mut own: Vec<&FrameHitch>,
    hiccups: &[&ProbeHiccup],
    capture: &DeepCapture,
) -> (Suspect, f64) {
    own.sort_by(|a, b| {
        b.frame_ms
            .total_cmp(&a.frame_ms)
            .then(a.ts_ms.cmp(&b.ts_ms))
    });
    let backing: Vec<&ProbeHiccup> = hiccups
        .iter()
        .copied()
        .filter(|h| own.iter().any(|f| (f.ts_ms - h.ts_ms).abs() <= NEAR_MS))
        .collect();
    let summary = capture
        .apps
        .iter()
        .filter(|a| a.name == app)
        .max_by_key(|a| a.hitches);
    let mut evidence: Vec<Evidence> = summary.map(frames_evidence).into_iter().collect();
    evidence.extend(own.iter().take(EVIDENCE_KEPT).copied().map(hitch_evidence));
    let confidence = if backing.is_empty() {
        Confidence::Medium
    } else {
        Confidence::High
    };
    evidence.extend(backing.into_iter().take(EVIDENCE_KEPT).map(hiccup_evidence));
    let suspect = Suspect {
        cause: Cause::Frames {
            app: app.to_string(),
        },
        confidence,
        evidence,
        next_steps: Vec::new(),
    };
    (suspect, own.len() as f64)
}

fn update_driver(intermediary: bool) -> Vec<NextStep> {
    if intermediary {
        Vec::new()
    } else {
        vec![NextStep::UpdateDriver]
    }
}

fn frames_evidence(app: &AppFrames) -> Evidence {
    Evidence::Frames {
        app: app.name.clone(),
        frames: app.frames,
        hitches: app.hitches,
        worst_ms: app.worst_ms,
        median_ms: app.median_ms,
    }
}

fn hitch_evidence(hitch: &FrameHitch) -> Evidence {
    Evidence::Hitch {
        ts_ms: hitch.ts_ms,
        app: hitch.app.clone(),
        frame_ms: hitch.frame_ms,
    }
}

fn hiccup_evidence(hiccup: &ProbeHiccup) -> Evidence {
    Evidence::Hiccup {
        ts_ms: hiccup.ts_ms,
        lag_ms: hiccup.lag_ms,
    }
}

/// Lost events cap every suspect at medium and are said on each; then most confident first, largest evidence next.
fn ranked(mut found: Vec<(Suspect, f64)>, lost_events: u32) -> Vec<Suspect> {
    if lost_events > 0 {
        for (suspect, _) in &mut found {
            suspect.confidence = suspect.confidence.min(Confidence::Medium);
            suspect.evidence.push(Evidence::CaptureLoss { lost_events });
        }
    }
    found.sort_by(|a, b| {
        b.0.confidence
            .cmp(&a.0.confidence)
            .then(b.1.total_cmp(&a.1))
    });
    found.into_iter().map(|(suspect, _)| suspect).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: i64 = 1_000_000;

    fn stats(durations: &[u32]) -> RoutineStats {
        let mut stats = RoutineStats::default();
        for us in durations {
            stats.add(*us);
        }
        stats
    }

    fn driver(module: &str, dpc: &[u32], isr: &[u32]) -> DriverLatency {
        DriverLatency {
            module: module.to_string(),
            dpc: stats(dpc),
            isr: stats(isr),
        }
    }

    fn long(ts_ms: i64, module: &str, us: u32) -> LongRoutine {
        LongRoutine {
            ts_ms,
            core: 0,
            module: module.to_string(),
            kind: RoutineKind::Dpc,
            us,
        }
    }

    fn hitch(ts_ms: i64, app: &str) -> FrameHitch {
        FrameHitch {
            ts_ms,
            app: app.to_string(),
            pid: 7,
            frame_ms: 120.0,
            median_ms: 16.7,
        }
    }

    fn app(name: &str, hitches: u32) -> AppFrames {
        AppFrames {
            name: name.to_string(),
            pid: 7,
            frames: 3_600,
            median_ms: 16.7,
            p99_ms: 20.0,
            worst_ms: 120.0,
            hitches,
        }
    }

    /// A one-minute capture with nothing in it.
    fn capture() -> DeepCapture {
        DeepCapture {
            start_ms: START,
            end_ms: START + 60_000,
            kernel: CaptureSource::Ok,
            frames: CaptureSource::Ok,
            lost_events: 0,
            drivers: Vec::new(),
            longest: Vec::new(),
            apps: Vec::new(),
            hitches: Vec::new(),
            hiccups: Vec::new(),
        }
    }

    fn named<'a>(found: &'a [Suspect], module: &str) -> Option<&'a Suspect> {
        found
            .iter()
            .find(|s| matches!(&s.cause, Cause::DriverModule { module: m, .. } if m == module))
    }

    #[test]
    fn buckets_follow_tracerpt_bounds() {
        assert_eq!(
            BUCKET_BOUNDS_US,
            [1, 2, 3, 4, 5, 10, 25, 50, 100, 250, 500, 1000]
        );
        assert_eq!(BUCKETS, BUCKET_BOUNDS_US.len() + 1);
        let cases: [(u32, usize); 12] = [
            (0, 0),
            (1, 1),
            (4, 4),
            (5, 5),
            (24, 6),
            (25, 7),
            (99, 8),
            (100, 9),
            (499, 10),
            (500, 11),
            (999, 11),
            (1_000, 12),
        ];
        for (us, bucket) in cases {
            let one = stats(&[us]);
            assert_eq!(one.buckets[bucket], 1, "{us} us");
            assert_eq!(one.buckets.iter().sum::<u32>(), 1, "{us} us");
        }
        assert_eq!(stats(&[5_000_000]).buckets[12], 1);
    }

    #[test]
    fn at_least_counts_whole_buckets_from_a_bound() {
        let s = stats(&[0, 99, 100, 250, 999, 1_000, 40_000]);
        assert_eq!(s.at_least(0), 7);
        assert_eq!(s.at_least(100), 5);
        assert_eq!(s.at_least(1_000), 2);
        let mut merged = s.clone();
        merged.merge(&stats(&[2_000]));
        assert_eq!(
            (merged.count, merged.max_us, merged.at_least(1_000)),
            (8, 40_000, 3)
        );
        assert_eq!(merged.total_us, s.total_us + 2_000);
    }

    #[test]
    fn module_map_finds_the_image_of_an_address() {
        let mut map = ModuleMap::new();
        map.insert(0x9000, 0x1000, r"\SystemRoot\System32\drivers\storport.sys");
        map.insert(0x1000, 0x2000, "C:/Windows/System32/ntoskrnl.exe");
        assert_eq!(map.find(0x1000), Some("ntoskrnl.exe"));
        assert_eq!(map.find(0x2fff), Some("ntoskrnl.exe"));
        assert_eq!(map.find(0x3000), None);
        assert_eq!(map.find(0x9500), Some("storport.sys"));
        assert_eq!(map.find(0xa000), None);
        assert_eq!(map.find(0x10), None);
        map.insert(0x9000, 0x100, "ndis.sys");
        assert_eq!(map.find(0x9050), Some("ndis.sys"));
        assert_eq!(map.find(0x9500), None);
    }

    #[test]
    fn routines_fold_into_modules() {
        let mut map = ModuleMap::new();
        map.insert(0x1000, 0x1000, "a.sys");
        map.insert(0x3000, 0x1000, "b.sys");
        let mut agg = RoutineAggregator::new();
        agg.add(0x1100, RoutineKind::Dpc, 10, 0, 1);
        agg.add(0x1200, RoutineKind::Dpc, 30, 1, 2);
        agg.add(0x1200, RoutineKind::Isr, 5, 1, 3);
        agg.add(0x3100, RoutineKind::Dpc, 100, 0, 4);
        agg.add(0xdead_0000, RoutineKind::Isr, 7, 0, 5);
        let (drivers, longest) = agg.finish(&map);
        let order: Vec<&str> = drivers.iter().map(|d| d.module.as_str()).collect();
        assert_eq!(order, ["b.sys", "a.sys", "<unknown>"]);
        let a = &drivers[1];
        assert_eq!((a.dpc.count, a.dpc.total_us, a.dpc.max_us), (2, 40, 30));
        assert_eq!((a.isr.count, a.isr.total_us), (1, 5));
        assert_eq!(drivers[2].isr.count, 1);
        assert_eq!(longest[0].module, "b.sys");
        assert_eq!(longest.last().map(|l| l.module.as_str()), Some("a.sys"));
    }

    #[test]
    fn longest_keeps_the_top_fifty() {
        let mut agg = RoutineAggregator::new();
        for us in 1..=80u32 {
            agg.add(0x10, RoutineKind::Dpc, us, 0, i64::from(us));
        }
        let (drivers, longest) = agg.finish(&ModuleMap::new());
        assert_eq!(longest.len(), LONGEST_KEPT);
        assert_eq!(longest.first().map(|l| l.us), Some(80));
        assert_eq!(longest.last().map(|l| l.us), Some(31));
        assert!(longest.windows(2).all(|w| w[0].us >= w[1].us));
        assert_eq!(drivers[0].dpc.count, 80);
    }

    #[test]
    fn driver_over_1ms_is_high() {
        let mut c = capture();
        c.drivers = vec![driver("nvlddmkm.sys", &[20, 1_500], &[])];
        let found = capture_findings(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].confidence, Confidence::High);
        assert_eq!(
            found[0].cause,
            Cause::DriverModule {
                module: "nvlddmkm.sys".to_string(),
                intermediary: false
            }
        );
        assert_eq!(found[0].next_steps, [NextStep::UpdateDriver]);
        assert!(matches!(
            found[0].evidence[0],
            Evidence::Routine {
                max_us: 1_500,
                count: 2,
                at_least_1ms: 1,
                ..
            }
        ));
    }

    #[test]
    fn frequent_slow_routines_make_a_medium_suspect() {
        let mut c = capture();
        c.drivers = vec![
            driver("busy.sys", &[150; 60], &[]),
            driver("calm.sys", &[150; 59], &[]),
            driver("heavy.sys", &[900; 1_500], &[]),
        ];
        let found = capture_findings(&c);
        assert_eq!(found.len(), 2);
        assert!(named(&found, "busy.sys").is_some_and(|s| s.confidence == Confidence::Medium));
        // 1,500 routines of 900 us in a minute take 2.25% of a core.
        assert!(named(&found, "heavy.sys").is_some_and(|s| s.confidence == Confidence::Medium));
        assert!(named(&found, "calm.sys").is_none());
    }

    #[test]
    fn intermediary_is_never_high_alone() {
        assert!(is_intermediary("StorPort.sys"));
        assert!(!is_intermediary("storahci.sys"));
        let mut c = capture();
        c.drivers = vec![
            driver("storport.sys", &[5_000], &[]),
            driver("ndis.sys", &[150; 60], &[]),
        ];
        let found = capture_findings(&c);
        let storport = named(&found, "storport.sys").expect("storport");
        assert_eq!(storport.confidence, Confidence::Medium);
        assert!(storport.next_steps.is_empty());
        assert_eq!(
            named(&found, "ndis.sys").map(|s| s.confidence),
            Some(Confidence::Low)
        );

        c.longest = vec![long(START + 5_000, "storport.sys", 5_000)];
        let alone = capture_suspects(START, START + 60_000, &c);
        assert_eq!(alone.len(), 1);
        assert_eq!(alone[0].confidence, Confidence::Medium);
    }

    #[test]
    fn kernel_core_is_never_a_suspect() {
        assert!(is_kernel_core("NTOSKRNL.EXE"));
        assert!(is_kernel_core("hal.dll"));
        assert!(!is_kernel_core("storport.sys"));
        let mut c = capture();
        c.drivers = vec![
            driver("ntoskrnl.exe", &[9_000], &[]),
            driver("hal.dll", &[9_000], &[]),
            driver("<unknown>", &[9_000], &[]),
        ];
        c.longest = vec![
            long(START + 10, "ntoskrnl.exe", 9_000),
            long(START + 20, "<unknown>", 9_000),
        ];
        assert!(capture_findings(&c).is_empty());
        assert!(capture_suspects(START, START + 60_000, &c).is_empty());
    }

    #[test]
    fn hitch_needs_2_5x_the_median_and_20ms() {
        let mut agg = FrameAggregator::new();
        for i in 0..100 {
            agg.add("steady", 1, i, 8.0);
            agg.add("fast", 2, i, 4.0);
            agg.add("slow", 3, i, 16.0);
        }
        agg.add("steady", 1, 500, 20.0);
        agg.add("steady", 1, 600, 20.5);
        agg.add("fast", 2, 700, 15.0);
        agg.add("slow", 3, 800, 39.0);
        agg.add("slow", 3, 900, 41.0);
        agg.add("few", 4, 1, 500.0);
        let (apps, hitches) = agg.finish();
        let times: Vec<(i64, &str)> = hitches.iter().map(|h| (h.ts_ms, h.app.as_str())).collect();
        assert_eq!(times, [(600, "steady"), (900, "slow")]);
        assert_eq!(apps.len(), 3, "an app under 60 frames is left out");
        assert_eq!(apps[0].hitches, 1);
        let fast = apps.iter().find(|a| a.name == "fast").expect("fast");
        assert_eq!((fast.hitches, fast.frames, fast.median_ms), (0, 101, 4.0));
        assert_eq!(fast.worst_ms, 15.0);
    }

    #[test]
    fn frames_are_summarised_and_hitches_kept_worst_first() {
        let mut agg = FrameAggregator::new();
        for i in 0..5_000 {
            agg.add("game", 1, i, 10.0);
        }
        for i in 0..HITCHES_KEPT as i64 + 20 {
            agg.add("game", 1, 10_000 + i, 50.0 + i as f32);
        }
        let (apps, hitches) = agg.finish();
        assert_eq!(apps[0].hitches, HITCHES_KEPT as u32 + 20);
        assert_eq!(hitches.len(), HITCHES_KEPT);
        assert!(hitches.windows(2).all(|w| w[0].ts_ms <= w[1].ts_ms));
        assert!(hitches.iter().all(|h| h.frame_ms >= 70.0));
        assert!(apps[0].p99_ms > apps[0].median_ms);
    }

    #[test]
    fn app_with_three_hitches_is_a_frames_suspect() {
        let mut c = capture();
        c.apps = vec![app("game.exe", 3), app("quiet.exe", 2)];
        c.hitches = (0..8)
            .map(|i| hitch(START + i * 1_000, "game.exe"))
            .collect();
        let found = capture_findings(&c);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].cause,
            Cause::Frames {
                app: "game.exe".to_string()
            }
        );
        assert_eq!(found[0].confidence, Confidence::Medium);
        let hitch_count = found[0]
            .evidence
            .iter()
            .filter(|e| matches!(e, Evidence::Hitch { .. }))
            .count();
        assert_eq!(hitch_count, EVIDENCE_KEPT);
        assert!(matches!(
            found[0].evidence[0],
            Evidence::Frames { hitches: 3, .. }
        ));
    }

    #[test]
    fn suspects_only_inside_the_window() {
        let mut c = capture();
        c.longest = vec![
            long(START + 1_000, "early.sys", 3_000),
            long(START + 30_000, "inside.sys", 2_000),
            long(START + 55_000, "late.sys", 4_000),
            long(START + 31_000, "short.sys", 900),
        ];
        c.hitches = vec![
            hitch(START + 1_000, "early.exe"),
            hitch(START + 30_500, "inside.exe"),
        ];
        let found = capture_suspects(START + 29_000, START + 32_000, &c);
        assert_eq!(found.len(), 2);
        assert!(named(&found, "inside.sys").is_some());
        assert!(found.iter().any(|s| s.cause
            == Cause::Frames {
                app: "inside.exe".to_string()
            }));
        assert!(capture_suspects(START + 40_000, START + 50_000, &c).is_empty());
    }

    #[test]
    fn long_routine_next_to_a_hiccup_is_high() {
        let at = START + 20_000;
        let mut c = capture();
        c.longest = vec![long(at, "storport.sys", 2_000)];
        c.hiccups = vec![ProbeHiccup {
            ts_ms: at + 60,
            lag_ms: 30.0,
        }];
        let far = capture_suspects(START, START + 60_000, &c);
        assert_eq!(far[0].confidence, Confidence::Medium);
        c.hiccups = vec![ProbeHiccup {
            ts_ms: at + 50,
            lag_ms: 30.0,
        }];
        let near = capture_suspects(START, START + 60_000, &c);
        assert_eq!(near[0].confidence, Confidence::High);
        assert!(
            near[0]
                .evidence
                .iter()
                .any(|e| matches!(e, Evidence::Hiccup { .. }))
        );
        c.hiccups.clear();
        c.hitches = vec![hitch(at - 40, "game.exe")];
        let found = capture_suspects(START, START + 60_000, &c);
        assert_eq!(
            named(&found, "storport.sys").map(|s| s.confidence),
            Some(Confidence::High)
        );
    }

    #[test]
    fn hitch_next_to_a_hiccup_is_high() {
        let at = START + 20_000;
        let mut c = capture();
        c.hitches = vec![hitch(at, "game.exe")];
        assert_eq!(
            capture_suspects(START, START + 60_000, &c)[0].confidence,
            Confidence::Medium
        );
        c.hiccups = vec![ProbeHiccup {
            ts_ms: at - 30,
            lag_ms: 25.0,
        }];
        assert_eq!(
            capture_suspects(START, START + 60_000, &c)[0].confidence,
            Confidence::High
        );
    }

    #[test]
    fn lost_events_cap_confidence_at_medium() {
        let mut c = capture();
        c.lost_events = 12;
        c.drivers = vec![driver("nvlddmkm.sys", &[5_000], &[])];
        c.apps = vec![app("game.exe", 3)];
        c.longest = vec![long(START + 100, "nvlddmkm.sys", 5_000)];
        c.hitches = vec![hitch(START + 120, "game.exe")];
        c.hiccups = vec![ProbeHiccup {
            ts_ms: START + 110,
            lag_ms: 40.0,
        }];
        for found in [
            capture_findings(&c),
            capture_suspects(START, START + 60_000, &c),
        ] {
            assert!(!found.is_empty());
            for s in &found {
                assert!(s.confidence <= Confidence::Medium);
                assert!(
                    s.evidence
                        .iter()
                        .any(|e| matches!(e, Evidence::CaptureLoss { lost_events: 12 }))
                );
            }
        }
    }

    #[test]
    fn every_capture_suspect_carries_evidence() {
        let mut c = capture();
        c.drivers = vec![
            driver("nvlddmkm.sys", &[5_000], &[]),
            driver("storport.sys", &[150; 100], &[]),
            driver("usbxhci.sys", &[], &[300; 80]),
        ];
        c.apps = vec![app("game.exe", 4)];
        c.longest = vec![long(START + 100, "nvlddmkm.sys", 5_000)];
        c.hitches = vec![hitch(START + 120, "game.exe")];
        for found in [
            capture_findings(&c),
            capture_suspects(START, START + 60_000, &c),
        ] {
            assert!(found.len() >= 2);
            assert!(found.iter().all(|s| !s.evidence.is_empty()));
        }
    }

    #[test]
    fn findings_come_most_confident_then_largest_first() {
        let mut c = capture();
        c.drivers = vec![
            driver("mid.sys", &[150; 60], &[]),
            driver("small.sys", &[1_200], &[]),
            driver("big.sys", &[8_000], &[]),
        ];
        c.apps = vec![app("game.exe", 3)];
        let order: Vec<Cause> = capture_findings(&c).into_iter().map(|s| s.cause).collect();
        let module = |m: &str| Cause::DriverModule {
            module: m.to_string(),
            intermediary: false,
        };
        assert_eq!(
            order,
            [
                module("big.sys"),
                module("small.sys"),
                module("mid.sys"),
                Cause::Frames {
                    app: "game.exe".to_string()
                },
            ]
        );
    }

    #[test]
    fn capture_without_frames_still_names_drivers() {
        let mut c = capture();
        c.frames = CaptureSource::Unavailable("presentmon missing".to_string());
        c.drivers = vec![driver("nvlddmkm.sys", &[3_000], &[])];
        c.longest = vec![long(START + 5_000, "nvlddmkm.sys", 3_000)];
        assert_eq!(capture_findings(&c).len(), 1);
        let window = capture_suspects(START, START + 60_000, &c);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].confidence, Confidence::High);
    }

    #[test]
    fn capture_roundtrips_through_postcard() {
        let mut c = capture();
        c.kernel = CaptureSource::Unavailable("denied".to_string());
        c.drivers = vec![driver("a.sys", &[1, 2, 3], &[4])];
        c.longest = vec![long(START + 1, "a.sys", 4)];
        c.apps = vec![app("game.exe", 1)];
        c.hitches = vec![hitch(START + 2, "game.exe")];
        c.hiccups = vec![ProbeHiccup {
            ts_ms: START + 3,
            lag_ms: 9.5,
        }];
        let bytes = postcard::to_stdvec(&c).expect("encode");
        assert_eq!(
            postcard::from_bytes::<DeepCapture>(&bytes).expect("decode"),
            c
        );
    }

    #[test]
    fn old_occurrence_bytes_still_decode() {
        // A suspect encoded before the capture variants existed.
        let bytes = [
            4, 0, 1, 1, 1, 128, 160, 171, 254, 249, 98, 6, 0, 0, 42, 66, 0, 0, 200, 65, 1, 1, 48,
            1, 9,
        ];
        let suspect: Suspect = postcard::from_bytes(&bytes).expect("decode");
        assert_eq!(suspect.cause, Cause::Driver { core: 0 });
        assert_eq!(suspect.confidence, Confidence::Medium);
        assert_eq!(suspect.next_steps, [NextStep::DeepCapture]);
        assert!(matches!(
            suspect.evidence[0],
            Evidence::Metric {
                ts_ms: 1_700_000_000_000,
                subject: Some(_),
                ..
            }
        ));
        assert_eq!(postcard::to_stdvec(&suspect).expect("encode"), bytes);
    }

    #[test]
    fn compositor_and_on_demand_apps_are_left_out() {
        let mut agg = FrameAggregator::new();
        for i in 0..200 {
            agg.add("dwm.exe", 10, i * 7, 7.0);
            agg.add("game.exe", 11, i * 7, 7.0);
            agg.add("browser.exe", 12, i * 7, 7.0);
        }
        agg.add("dwm.exe", 10, 5_000, 1_000.0);
        agg.add("game.exe", 11, 5_000, 120.0);
        for i in 0..20 {
            agg.add("browser.exe", 12, 6_000 + i * 1_000, 1_000.0);
        }
        let (apps, hitches) = agg.finish();
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["game.exe"]);
        assert!(hitches.iter().all(|h| h.app == "game.exe"));
    }

    #[test]
    fn stored_capture_drops_the_compositor() {
        let mut stored = capture();
        stored.apps.push(AppFrames {
            name: "dwm.exe".into(),
            pid: 99,
            frames: 29_839,
            median_ms: 5.6,
            p99_ms: 900.0,
            worst_ms: 1_008.0,
            hitches: 2_374,
        });
        stored.hitches.push(FrameHitch {
            ts_ms: stored.start_ms + 1_000,
            app: "dwm.exe".into(),
            pid: 99,
            frame_ms: 1_008.0,
            median_ms: 5.6,
        });
        let steady = steady_only(&stored);
        assert!(steady.apps.iter().all(|a| a.name != "dwm.exe"));
        assert!(steady.hitches.iter().all(|h| h.app != "dwm.exe"));
        let named = capture_findings(&stored);
        assert!(
            !named
                .iter()
                .any(|s| matches!(&s.cause, Cause::Frames { app } if app == "dwm.exe"))
        );
    }
}
