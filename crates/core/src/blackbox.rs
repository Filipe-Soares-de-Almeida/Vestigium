use std::collections::{HashSet, VecDeque};

use crate::{
    Cause, Episode, EpisodeDetector, EpisodeKind, EventMeaning, Gap, GapReason, HUNG_AFTER_MS,
    HangSpan, HangTracker, HungWindow, MachineInfo, Occurrence, OccurrenceKind, PressureSpan,
    ProbeTick, ProcessTop, Ring, Sample, Severity, StutterDetector, Trigger, WinEvent, Window,
    classify, facts, freeze, severity, suspects,
};

/// How much the rings keep.
pub const RING_MS: i64 = 10 * 60_000;
/// After a resume, the probe and the counters settle for this long.
pub const GAP_MARGIN_MS: i64 = 2_000;
/// A hung app first seen this soon after a freeze is the freeze's echo.
pub const HANG_ECHO_MS: i64 = 10_000;
/// Freezes shorter than this leave apps responsive.
pub const ECHOING_FREEZE_MS: i64 = 1_000;
const DAY_MS: i64 = 24 * 60 * 60_000;

/// How an occurrence's window is cut and when close triggers join one occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    pub before_ms: i64,
    pub after_ms: i64,
    /// A trigger starting this soon after a pending occurrence ends joins it.
    pub merge_ms: i64,
    /// An occurrence never spans more than this.
    pub max_ms: i64,
}

impl Default for Timing {
    fn default() -> Timing {
        Timing {
            before_ms: 60_000,
            after_ms: 30_000,
            merge_ms: 30_000,
            max_ms: 5 * 60_000,
        }
    }
}

/// What the black box hands over to be stored.
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    Occurrence(Box<Occurrence>),
    Episode(Episode),
    Gap(Gap),
}

/// How much happened in the last 24 h, for the tray.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecentState {
    pub occurrences: u32,
    /// Occurrences rated severe.
    pub severe: u32,
    /// Pressure episodes; hints are left out.
    pub episodes: u32,
    /// Health findings rated critical that are open now.
    pub critical_findings: u32,
}

/// Something counted by `RecentState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notable {
    Occurrence { severe: bool },
    Episode,
}

#[derive(Debug)]
struct Pending {
    start_ms: i64,
    end_ms: i64,
    triggers: Vec<Trigger>,
    sensitive: bool,
}

/// Everything the agent sees goes in; everything worth storing comes out.
#[derive(Debug)]
pub struct Blackbox {
    machine: MachineInfo,
    timing: Timing,
    samples: Ring<Sample>,
    tops: Ring<ProcessTop>,
    events: Ring<WinEvent>,
    hangs: Ring<HangSpan>,
    gaps: Ring<Gap>,
    /// Freezes long enough to make every app look hung, as (start, end).
    freezes: Ring<(i64, i64)>,
    /// Pressure episodes that ended; hints are left out.
    pressure: Ring<PressureSpan>,
    stutter: StutterDetector,
    /// Stutters emitted since sensitive mode went on.
    stutters: u32,
    /// The first of those blamed a driver or nothing clear.
    first_stutter_open: bool,
    hang_tracker: HangTracker,
    episodes: EpisodeDetector,
    sensitive: bool,
    pending: Vec<Pending>,
    ready: Vec<Output>,
    notable: VecDeque<(i64, Notable)>,
}

impl Blackbox {
    pub fn new(machine: MachineInfo, timing: Timing) -> Blackbox {
        Blackbox {
            machine,
            timing,
            samples: Ring::with_capacity(660),
            tops: Ring::with_capacity(140),
            events: Ring::with_capacity(512),
            hangs: Ring::with_capacity(256),
            gaps: Ring::with_capacity(64),
            freezes: Ring::with_capacity(64),
            pressure: Ring::with_capacity(64),
            stutter: StutterDetector::new(),
            stutters: 0,
            first_stutter_open: false,
            hang_tracker: HangTracker::new(),
            episodes: EpisodeDetector::new(),
            sensitive: false,
            pending: Vec::new(),
            ready: Vec::new(),
            notable: VecDeque::new(),
        }
    }

    /// A black box for the same machine and timing that remembers nothing, for a cleared history.
    pub fn emptied(&self) -> Blackbox {
        Blackbox::new(self.machine.clone(), self.timing)
    }

    /// Updates the machine facts later windows carry, such as once the disks are known.
    pub fn set_machine(&mut self, machine: MachineInfo) {
        self.machine = machine;
    }

    pub fn machine(&self) -> &MachineInfo {
        &self.machine
    }

    pub fn push_probe(&mut self, tick: ProbeTick) {
        if let Some(trigger) = freeze(&tick) {
            if tick.lag_ms >= ECHOING_FREEZE_MS as f32 {
                self.freezes.push((trigger.start_ms(), trigger.end_ms()));
            }
            self.add(trigger);
        }
        if self.sensitive
            && let Some(trigger) = self.stutter.push(&tick)
        {
            self.add(trigger);
        }
    }

    pub fn push_sample(&mut self, sample: Sample) {
        let ended = self.episodes.push_sample(&sample, self.sensitive);
        self.ready_episodes(ended);
        self.samples.push(sample);
    }

    pub fn push_top(&mut self, top: ProcessTop) {
        self.episodes.push_top(&top);
        self.tops.push(top);
    }

    /// Keeps the event; a storage reset, a display reset or a hardware error is an occurrence by itself.
    pub fn push_event(&mut self, event: WinEvent) {
        let watched_since = self.samples.iter().next().map(|s| s.ts_ms);
        let reported = matches!(
            classify(event.channel, &event.provider, event.id),
            Some(
                EventMeaning::StorageReset
                    | EventMeaning::DiskRetry
                    | EventMeaning::DisplayReset
                    | EventMeaning::HardwareError
            )
        );
        if reported && watched_since.is_some_and(|since| event.ts_ms >= since) {
            self.add(Trigger::Reported {
                event: event.clone(),
            });
        }
        self.events.push(event);
    }

    /// Feeds one not-responding check; `running` holds every live pid.
    pub fn push_hangs(&mut self, ts_ms: i64, hung: &[HungWindow], running: &HashSet<u32>) {
        for trigger in self.hang_tracker.push(ts_ms, hung, running) {
            let Trigger::AppHang {
                pid,
                name,
                start_ms,
                end_ms,
                outcome,
            } = &trigger
            else {
                continue;
            };
            self.hangs.push(HangSpan {
                pid: *pid,
                name: name.clone(),
                start_ms: *start_ms,
                end_ms: *end_ms,
                outcome: *outcome,
            });
            let first_seen = start_ms + HUNG_AFTER_MS;
            let echo = self
                .freezes
                .iter()
                .any(|&(start, end)| (start..=end + HANG_ECHO_MS).contains(&first_seen));
            if !echo {
                self.add(trigger);
            }
        }
    }

    /// Cuts every gap that ends after `now_ms` down to it and removes the ones that start after it,
    /// so a gap stamped while the wall clock was ahead stops covering the time after it stepped back.
    pub fn cut_gaps_to(&mut self, now_ms: i64) {
        self.gaps.retain_mut(|gap| {
            gap.end_ms = gap.end_ms.min(now_ms);
            gap.start_ms <= now_ms
        });
    }

    /// Records a hole in watching; triggers inside it, or just after it, are dropped.
    pub fn push_gap(&mut self, gap: Gap) {
        self.gaps.push(gap);
        for pending in &mut self.pending {
            pending.triggers.retain(|t| !touches(t, &gap));
        }
        self.pending.retain(|p| !p.triggers.is_empty());
        for pending in &mut self.pending {
            pending.start_ms = pending
                .triggers
                .iter()
                .map(Trigger::start_ms)
                .min()
                .unwrap_or(0);
            pending.end_ms = pending
                .triggers
                .iter()
                .map(Trigger::end_ms)
                .max()
                .unwrap_or(0);
        }
        self.ready.push(Output::Gap(gap));
    }

    pub fn set_sensitive(&mut self, on: bool) {
        if !on {
            self.stutter.reset();
            self.stutters = 0;
            self.first_stutter_open = false;
        }
        self.sensitive = on;
    }

    /// Whether the stutters of this sensitive-mode session call for a deep capture.
    pub fn suggests_capture(&self) -> bool {
        self.sensitive && (self.stutters >= 2 || (self.stutters == 1 && self.first_stutter_open))
    }

    /// Occurrences whose window after has passed, finished episodes and recorded gaps.
    pub fn poll(&mut self, now_ms: i64) -> Vec<Output> {
        let after_ms = self.timing.after_ms;
        let (due, waiting): (Vec<Pending>, Vec<Pending>) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|p| p.end_ms + after_ms <= now_ms);
        self.pending = waiting;
        for pending in due {
            self.finish(pending, now_ms, false);
        }
        self.forget(now_ms);
        std::mem::take(&mut self.ready)
    }

    /// Closes everything, as when watching pauses, sleeps or stops.
    pub fn drain(&mut self, now_ms: i64) -> Vec<Output> {
        for pending in std::mem::take(&mut self.pending) {
            self.finish(pending, now_ms, true);
        }
        let closed = self.episodes.close_all();
        self.ready_episodes(closed);
        self.hang_tracker.reset();
        self.stutter.reset();
        std::mem::take(&mut self.ready)
    }

    /// Closes what is open and forgets everything stamped before the wall clock moved from
    /// `from_ms` to `to_ms`, then records the step as a gap; a step back is a gap of no width.
    pub fn clock_changed(&mut self, from_ms: i64, to_ms: i64) -> Vec<Output> {
        let mut outputs = self.drain(from_ms);
        self.samples.clear();
        self.tops.clear();
        self.events.clear();
        self.hangs.clear();
        self.freezes.clear();
        self.gaps.clear();
        self.pressure.clear();
        let (start_ms, end_ms) = if to_ms >= from_ms {
            (from_ms, to_ms)
        } else {
            (to_ms, to_ms)
        };
        self.push_gap(Gap {
            start_ms,
            end_ms,
            reason: GapReason::ClockChanged,
        });
        outputs.append(&mut self.ready);
        outputs
    }

    /// The CPU, disk and memory pressure episodes under way; hints are left out.
    pub fn open_pressure(&self) -> Vec<EpisodeKind> {
        self.episodes.open_pressure()
    }

    /// The pressure episodes of the rings' span, those that ended and then those under way.
    pub fn recent_pressure(&self) -> Vec<PressureSpan> {
        self.pressure
            .iter()
            .cloned()
            .chain(self.episodes.open_spans())
            .collect()
    }

    /// Counts what happened in the 24 h before `now_ms`.
    pub fn recent(&self, now_ms: i64) -> RecentState {
        let mut state = RecentState::default();
        for (_, notable) in self.notable.iter().filter(|(at, _)| now_ms - at < DAY_MS) {
            match notable {
                Notable::Occurrence { severe } => {
                    state.occurrences += 1;
                    state.severe += u32::from(*severe);
                }
                Notable::Episode => state.episodes += 1,
            }
        }
        state
    }

    /// Counts something stored before this black box started, such as on the agent's last run.
    pub fn remember(&mut self, at_ms: i64, notable: Notable) {
        self.notable.push_back((at_ms, notable));
    }

    /// Takes back the count of an occurrence or episode that could not be stored.
    pub fn forget_notable(&mut self, output: &Output) {
        let counted = match output {
            Output::Occurrence(occurrence) => Some((
                occurrence.end_ms,
                Notable::Occurrence {
                    severe: occurrence.severity == Severity::Severe,
                },
            )),
            Output::Episode(episode)
                if !matches!(
                    episode.kind,
                    EpisodeKind::DpcIsrHint { .. } | EpisodeKind::DiskLatencyHint { .. }
                ) =>
            {
                Some((episode.end_ms, Notable::Episode))
            }
            _ => None,
        };
        if let Some(counted) = counted
            && let Some(at) = self.notable.iter().position(|n| *n == counted)
        {
            self.notable.remove(at);
        }
    }

    pub fn samples(&self) -> impl DoubleEndedIterator<Item = &Sample> {
        self.samples.iter()
    }

    pub fn tops(&self) -> impl DoubleEndedIterator<Item = &ProcessTop> {
        self.tops.iter()
    }

    fn add(&mut self, trigger: Trigger) {
        if self.gaps.iter().any(|gap| touches(&trigger, gap)) {
            return;
        }
        let (start, end) = (trigger.start_ms(), trigger.end_ms());
        let timing = self.timing;
        let joins = self.pending.iter_mut().find(|p| {
            start <= p.end_ms + timing.merge_ms
                && end >= p.start_ms - timing.merge_ms
                && end.max(p.end_ms) - start.min(p.start_ms) <= timing.max_ms
        });
        match joins {
            Some(pending) => {
                pending.start_ms = pending.start_ms.min(start);
                pending.end_ms = pending.end_ms.max(end);
                pending.sensitive |= self.sensitive;
                pending.triggers.push(trigger);
            }
            None => self.pending.push(Pending {
                start_ms: start,
                end_ms: end,
                triggers: vec![trigger],
                sensitive: self.sensitive,
            }),
        }
    }

    fn finish(&mut self, pending: Pending, now_ms: i64, truncated: bool) {
        let Pending {
            start_ms,
            end_ms,
            mut triggers,
            sensitive,
        } = pending;
        triggers.sort_by_key(Trigger::start_ms);
        let from_ms = start_ms - self.timing.before_ms;
        let to_ms = (end_ms + self.timing.after_ms).min(now_ms.max(end_ms));
        let inside = |ts: i64| (from_ms..=to_ms).contains(&ts);
        let overlaps = |start: i64, end: i64| start <= to_ms && end >= from_ms;
        let window = Window {
            from_ms,
            to_ms,
            samples: self
                .samples
                .iter()
                .filter(|s| inside(s.ts_ms))
                .cloned()
                .collect(),
            tops: self
                .tops
                .iter()
                .filter(|t| inside(t.ts_ms))
                .cloned()
                .collect(),
            events: self
                .events
                .iter()
                .filter(|e| inside(e.ts_ms))
                .cloned()
                .collect(),
            hangs: self
                .hangs
                .iter()
                .filter(|h| overlaps(h.start_ms, h.end_ms))
                .cloned()
                .collect(),
            gaps: self
                .gaps
                .iter()
                .filter(|g| overlaps(g.start_ms, g.end_ms))
                .copied()
                .collect(),
            machine: self.machine.clone(),
        };
        let kind = triggers
            .iter()
            .map(OccurrenceKind::of)
            .max_by_key(|k| k.rank())
            .unwrap_or(OccurrenceKind::Reported);
        let severity = triggers
            .iter()
            .map(|t| severity(stall_ms(t)))
            .max()
            .unwrap_or(Severity::Light);
        let occurrence = Occurrence {
            kind,
            start_ms,
            end_ms,
            severity,
            suspects: suspects(&triggers, &window),
            facts: facts(&triggers, &window),
            triggers,
            window,
            sensitive,
            truncated,
        };
        self.notable.push_back((
            end_ms,
            Notable::Occurrence {
                severe: severity == Severity::Severe,
            },
        ));
        if self.sensitive && occurrence.kind == OccurrenceKind::Stutter {
            if self.stutters == 0 {
                self.first_stutter_open =
                    matches!(occurrence.cause(), Cause::Driver { .. } | Cause::Unclear);
            }
            self.stutters += 1;
        }
        self.ready.push(Output::Occurrence(Box::new(occurrence)));
    }

    fn ready_episodes(&mut self, episodes: Vec<Episode>) {
        for episode in episodes {
            if !matches!(
                episode.kind,
                EpisodeKind::DpcIsrHint { .. } | EpisodeKind::DiskLatencyHint { .. }
            ) {
                self.notable.push_back((episode.end_ms, Notable::Episode));
                self.pressure.push(PressureSpan {
                    kind: episode.kind.clone(),
                    start_ms: episode.start_ms,
                    end_ms: Some(episode.end_ms),
                });
            }
            self.ready.push(Output::Episode(episode));
        }
    }

    /// Drops what has left the rings' span, and notables older than a day.
    fn forget(&mut self, now_ms: i64) {
        let cutoff_ms = now_ms - RING_MS;
        self.samples.drop_while(|s| s.ts_ms < cutoff_ms);
        self.tops.drop_while(|t| t.ts_ms < cutoff_ms);
        self.events.drop_while(|e| e.ts_ms < cutoff_ms);
        self.hangs.drop_while(|h| h.end_ms < cutoff_ms);
        self.gaps.drop_while(|g| g.end_ms < cutoff_ms);
        self.freezes.drop_while(|&(_, end)| end < cutoff_ms);
        self.pressure
            .drop_while(|span| span.end_ms.is_some_and(|end| end < cutoff_ms));
        while self
            .notable
            .front()
            .is_some_and(|(at, _)| now_ms - at >= DAY_MS)
        {
            self.notable.pop_front();
        }
    }
}

/// How long a trigger kept the person waiting, for its severity.
fn stall_ms(trigger: &Trigger) -> f32 {
    match trigger {
        Trigger::Freeze { lag_ms, .. } => *lag_ms,
        Trigger::Stutter { worst_ms, .. } => *worst_ms,
        Trigger::AppHang {
            start_ms, end_ms, ..
        } => (end_ms - start_ms) as f32,
        Trigger::Reported { .. } | Trigger::Crash { .. } => 0.0,
    }
}

/// Whether a trigger overlaps a gap or the settling time right after it.
fn touches(trigger: &Trigger, gap: &Gap) -> bool {
    trigger.start_ms() <= gap.end_ms + GAP_MARGIN_MS && trigger.end_ms() >= gap.start_ms
}
