use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use vestigium_collect::{
    BlackboxRing, CollectError, Delivered, EventSubscription, WindowsHealth, disk_media, hung_pids,
    logical_cpus, memory_info, physical_drive, unbiased_ms, wall_ms,
};
use vestigium_core::{
    Blackbox, CAPTURE_MINUTES_MAX, CpuWindow, DiskInfo, EventMeaning, Gap, GapReason, GateState,
    HICCUP_MS, HungWindow, MachineInfo, Media, Notable, OccurrenceSummary, Output, PressureSpan,
    ProbeHiccup, ProbeTick, ProcessCounters, ProcessTop, ProgramRing, RING_MS, Reading,
    RecentState, Ring, RunGate, Sample, Settings, Severity, Timing, WinEvent, heaviest, last_seen,
    process_rates, programs,
};
use vestigium_ipc::{AgentStatus, NowFrame};
use vestigium_store::{Batch, Batcher, Segment, Store, StoreError};
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

use vestigium_core::health::Concern;

use crate::app::lock;
use crate::clock::ClockWatch;
use crate::dumps::{DumpPlaces, take_inventory};
use crate::fall::{Fall, RingSync, Strain};
use crate::health::{HealthPass, PassNow};
use crate::power::PowerEvent;
use crate::readings::{Backoff, Readings, Source, SourceHealths, SourceSet, Waiter};
use crate::sensitive::SensitiveMode;
use crate::shutdown;

const TICK_MS: f64 = 1000.0;
/// Seconds between two stored process rankings.
const TOP_EVERY: usize = 5;
/// Seconds between two checks for windows that stopped responding.
const HANG_EVERY: usize = 2;
const TOP_PER_METRIC: usize = 5;
/// While paused, the longest wait before the end of a timed pause is checked again.
const PAUSE_CHECK_MS: i64 = 60_000;
/// The span the agent's own CPU share is averaged over.
const OWN_CPU_SPAN_MS: f64 = 60_000.0;
/// Programs the live screen can list in full, busiest first.
const PROGRAMS_LISTED: usize = 300;
/// Ticks after a start or a resume before idle pages are handed back to Windows.
const TRIM_AFTER_TICKS: usize = 10;
/// A stop shorter than this is a restart, not a gap.
const STOPPED_GAP_MIN_MS: i64 = 5_000;
const HOUR_MS: i64 = 60 * 60_000;
const DAY_MS: i64 = 24 * HOUR_MS;
/// A disk with I/O this recently is spinning, so a health read does not wake it.
const DISK_BUSY_MS: i64 = 60_000;
/// The first health pass waits this long after a start, which stays light.
const FIRST_HEALTH_AFTER_MS: i64 = 60_000;
/// How long a late probe wake stays available to a capture that ends later: the longest capture.
const HICCUPS_KEPT_MS: i64 = CAPTURE_MINUTES_MAX as i64 * 60_000;
const HICCUPS_KEPT: usize = 10_000;
/// Batches kept for another try after a failed write; past this the oldest loses its samples.
const BATCHES_KEPT: usize = 3;
/// Occurrences and episodes kept for another try after a failed write; each one is large.
const OUTPUTS_KEPT: usize = 5;

/// Lets the owner wait for the sampler to write its last batch and stop.
#[derive(Debug)]
pub struct SamplerHandle {
    done: Mutex<Receiver<()>>,
}

impl SamplerHandle {
    /// Waits until the sampler thread has ended, for at most `timeout`.
    pub fn wait(&self, timeout: Duration) -> bool {
        !matches!(
            lock(&self.done).recv_timeout(timeout),
            Err(mpsc::RecvTimeoutError::Timeout)
        )
    }
}

/// What the sampler is connected to.
pub struct Wiring {
    /// How often a batch is written.
    pub flush: Duration,
    pub timing: Timing,
    pub gate: Arc<RunGate>,
    pub probe_rx: Receiver<ProbeTick>,
    pub power_rx: Receiver<PowerEvent>,
    pub events_rx: Receiver<Delivered>,
    /// Kept alive as long as the sampler; `None` when the Event Log could not be read.
    pub subscription: Option<EventSubscription>,
    pub sensitive: Arc<SensitiveMode>,
    pub status: Arc<Mutex<AgentStatus>>,
    /// What the tray shows about the last 24 h.
    pub recent: Arc<Mutex<RecentState>>,
    /// The latest second watched, for the live screen.
    pub now: Arc<Mutex<NowFrame>>,
    /// The last ten minutes, one reading a second, for the live screen.
    pub readings: Arc<Mutex<Ring<Reading>>>,
    /// Each program's use at every ranking of the last ten minutes, for a program's page.
    pub programs: Arc<Mutex<ProgramRing>>,
    /// The pressure episodes of the last ten minutes, for the live screen.
    pub pressure: Arc<Mutex<Vec<PressureSpan>>>,
    /// Late probe wakes of the last fifteen minutes, kept while sensitive mode is on.
    pub hiccups: Arc<Mutex<VecDeque<ProbeHiccup>>>,
    /// Retention is read from here at every prune, so a change applies on the next one.
    pub settings: Arc<Mutex<Settings>>,
    /// Set by the pipe after it cleared the history.
    pub cleared: Arc<AtomicBool>,
    /// Set by the pipe after it stored a health scan.
    pub health_due: Arc<AtomicBool>,
    /// The black box file that outlives a fall; `None` keeps none.
    pub ring_path: Option<PathBuf>,
    /// What each tick reads from Windows, and how the counters and the timer are opened.
    pub source_set: SourceSet,
}

/// Starts the 1 Hz sampler, which feeds the black box and writes into `store`.
pub fn run_sampler(store: Store, wiring: Wiring) -> io::Result<SamplerHandle> {
    let (done_tx, done_rx) = mpsc::channel();
    thread::Builder::new()
        .name("sampler".to_string())
        .spawn(move || {
            let gate = wiring.gate.clone();
            let mut sampler = Sampler::new(store, wiring);
            sampler.run(&gate);
            sampler.take_events();
            sampler.settle_fall(wall_ms(), true);
            sampler.leave(GapReason::Stopped);
            sampler.close_ring();
            let _ = done_tx.send(());
        })?;
    Ok(SamplerHandle {
        done: Mutex::new(done_rx),
    })
}

/// True when the wall clock entered an hour that has not been pruned yet.
pub fn prune_due(pruned_hour: Option<i64>, now_ms: i64) -> bool {
    pruned_hour != Some(now_ms.div_euclid(HOUR_MS))
}

struct Sampler {
    store: Store,
    batcher: Batcher,
    blackbox: Blackbox,
    probe_rx: Receiver<ProbeTick>,
    power_rx: Receiver<PowerEvent>,
    events_rx: Receiver<Delivered>,
    _subscription: Option<EventSubscription>,
    sensitive: Arc<SensitiveMode>,
    status: Arc<Mutex<AgentStatus>>,
    recent: Arc<Mutex<RecentState>>,
    now: Arc<Mutex<NowFrame>>,
    readings: Arc<Mutex<Ring<Reading>>>,
    programs: Arc<Mutex<ProgramRing>>,
    pressure: Arc<Mutex<Vec<PressureSpan>>>,
    hiccups: Arc<Mutex<VecDeque<ProbeHiccup>>>,
    settings: Arc<Mutex<Settings>>,
    cleared: Arc<AtomicBool>,
    health_due: Arc<AtomicBool>,
    /// Events and the bookmark that covers them, waiting for the next batch.
    events: Vec<WinEvent>,
    bookmark_xml: Option<String>,
    gaps: Vec<Gap>,
    /// Batches whose write failed, oldest first, tried again with the next flush.
    waiting: VecDeque<Batch>,
    /// Occurrences and episodes whose write failed, oldest first, tried again at the next `keep`.
    unstored: VecDeque<Output>,
    /// How often a batch is written, and when the last one was.
    flush_ms: i64,
    last_write_ms: i64,
    /// When watching stopped and why, while paused or suspended.
    away_since: Option<(i64, GapReason)>,
    pruned_hour: Option<i64>,
    snapshots: VecDeque<(f64, Vec<ProcessCounters>)>,
    /// GPU use per process at the latest sample, for the next ranking.
    gpu_by_pid: HashMap<u32, f32>,
    logical_cpus: u32,
    /// The agent's own CPU time, for the share the status reports.
    own_cpu: CpuWindow,
    health: HealthPass<WindowsHealth>,
    /// When the first health pass is due; afterwards a pass runs with every hourly prune.
    first_health_ms: Option<i64>,
    /// When each disk last had I/O.
    disk_active_ms: HashMap<String, i64>,
    commit_limit_bytes: u64,
    /// Critical health findings open now, for the tray.
    critical_findings: u32,
    ring: Option<BlackboxRing>,
    ring_sync: RingSync,
    /// Tells a step of the wall clock apart from the unbiased clock.
    clock: ClockWatch,
    /// A stop that may have been a fall, until Windows says so.
    fall: Option<Fall>,
    /// Where the time away began, held until the fall is settled.
    held_gap: Option<i64>,
    started_ms: i64,
    /// Windows logged a hardware error since the start.
    hardware_error: bool,
    /// A crash was reported, so the dumps are listed again on the next tick.
    dumps_due: bool,
    /// What the sampler reads; taken by `run`.
    source_set: Option<SourceSet>,
    /// How each source has been reading.
    source_health: SourceHealths,
}

/// The data sources a tick reads, and the timer that paces it.
struct Sources {
    readings: Box<dyn Readings>,
    /// True once the performance counters opened; they are tried again until then.
    pdh_open: bool,
    pdh_backoff: Backoff,
    /// The high-resolution timer; ticks wait with a plain sleep while it is missing.
    timer: Option<Box<dyn Waiter>>,
    timer_backoff: Backoff,
}

impl Sources {
    fn new(readings: Box<dyn Readings>) -> Sources {
        Sources {
            readings,
            pdh_open: false,
            pdh_backoff: Backoff::new(),
            timer: None,
            timer_backoff: Backoff::new(),
        }
    }

    /// Takes the first readings, which only set the baseline for rates.
    fn prime(&mut self) {
        let _ = self.readings.cpu();
        self.prime_pdh();
    }

    fn prime_pdh(&mut self) {
        if self.pdh_open {
            let _ = self.readings.pdh();
        }
    }

    fn reset(&mut self) {
        self.readings.reset();
    }
}

impl Sampler {
    fn new(store: Store, wiring: Wiring) -> Sampler {
        let now = wall_ms();
        let opened = wiring.ring_path.as_deref().map(BlackboxRing::open);
        let machine = store.machine().ok().flatten().unwrap_or(MachineInfo {
            logical_cpus: logical_cpus(),
            mem_total_bytes: memory_info().map_or(0, |m| m.total_bytes),
            disks: Vec::new(),
        });
        let mut sampler = Sampler {
            batcher: Batcher::new(wiring.flush),
            blackbox: Blackbox::new(machine, wiring.timing),
            probe_rx: wiring.probe_rx,
            power_rx: wiring.power_rx,
            events_rx: wiring.events_rx,
            _subscription: wiring.subscription,
            sensitive: wiring.sensitive,
            status: wiring.status,
            recent: wiring.recent,
            now: wiring.now,
            readings: wiring.readings,
            programs: wiring.programs,
            pressure: wiring.pressure,
            hiccups: wiring.hiccups,
            settings: wiring.settings,
            cleared: wiring.cleared,
            health_due: wiring.health_due,
            events: Vec::new(),
            bookmark_xml: None,
            gaps: Vec::new(),
            waiting: VecDeque::new(),
            unstored: VecDeque::new(),
            flush_ms: i64::try_from(wiring.flush.as_millis()).unwrap_or(i64::MAX),
            last_write_ms: now,
            away_since: None,
            pruned_hour: None,
            snapshots: VecDeque::with_capacity(TOP_EVERY + 1),
            gpu_by_pid: HashMap::new(),
            logical_cpus: logical_cpus(),
            own_cpu: CpuWindow::new(OWN_CPU_SPAN_MS),
            health: HealthPass::new(WindowsHealth),
            first_health_ms: Some(now + FIRST_HEALTH_AFTER_MS),
            disk_active_ms: HashMap::new(),
            commit_limit_bytes: 0,
            critical_findings: 0,
            ring: None,
            ring_sync: RingSync::new(now),
            clock: ClockWatch::new(now, unbiased_ms()),
            fall: None,
            held_gap: None,
            started_ms: now,
            hardware_error: false,
            dumps_due: false,
            source_set: Some(wiring.source_set),
            source_health: SourceHealths::default(),
            store,
        };
        sampler.recall_last_day(now);
        let history_last = sampler.store.last_sample_end_ms().ok().flatten();
        match opened {
            Some(Ok((ring, held))) => {
                sampler.ring = Some(ring);
                sampler.fall = Fall::after(held, history_last, now);
            }
            Some(Err(err)) => sampler.report(&err),
            None => {}
        }
        let away_from = history_last.or_else(|| sampler.fall.as_ref().map(Fall::last_ms));
        if let Some(last) = away_from
            && now - last > STOPPED_GAP_MIN_MS
        {
            if sampler.fall.is_some() {
                sampler.held_gap = Some(last);
            } else {
                sampler.away_since = Some((last, GapReason::Stopped));
                sampler.come_back(now);
            }
        }
        sampler
    }

    /// Counts what the history already holds for the last 24 h, so the tray survives a restart.
    fn recall_last_day(&mut self, now: i64) {
        if let Ok(open) = self.store.open_findings() {
            self.critical_findings = open
                .iter()
                .filter(|f| f.finding.concern == Concern::Critical)
                .count() as u32;
        }
        if let Ok(occurrences) = self.store.occurrences(now - DAY_MS, now) {
            lock(&self.now).latest = occurrences.last().cloned();
            for o in occurrences {
                let severe = o.severity == Severity::Severe;
                self.blackbox
                    .remember(o.end_ms, Notable::Occurrence { severe });
            }
        }
        if let Ok(episodes) = self.store.episodes(now - DAY_MS, now) {
            for e in episodes.iter().filter(|e| !e.kind.is_hint()) {
                self.blackbox.remember(e.end_ms, Notable::Episode);
            }
        }
        self.publish_recent(now);
    }

    /// Ticks once a second until the gate says stop; pausing writes everything and blocks.
    /// Time runs on the unbiased clock, so a sleep of the whole system is not a late tick.
    fn run(&mut self, gate: &RunGate) {
        let Some(source_set) = self.source_set.take() else {
            return;
        };
        let mut sources = Sources::new(source_set());
        self.open_pdh(&mut sources);
        sources.prime();
        let mut next_tick = unbiased_ms() + TICK_MS;
        let mut ticks = 0usize;
        let mut trim_at = Some(TRIM_AFTER_TICKS);
        loop {
            match gate.state() {
                GateState::Stopping => return,
                GateState::Paused => {
                    self.leave(GapReason::Paused);
                    sources.reset();
                    self.snapshots.clear();
                    if self.wait_out_pause(gate) == GateState::Stopping {
                        return;
                    }
                    self.probe_rx.try_iter().for_each(drop);
                    self.come_back(wall_ms());
                    self.clock.rebase(wall_ms(), unbiased_ms());
                    sources.prime();
                    next_tick = unbiased_ms() + TICK_MS;
                    trim_at = Some(ticks + TRIM_AFTER_TICKS);
                    continue;
                }
                GateState::Running => {}
            }

            let due_ms = (next_tick - unbiased_ms()).max(0.0);
            self.pace(&mut sources, Duration::from_secs_f64(due_ms / 1000.0), gate);
            let woke = unbiased_ms();
            let sampler_lag_ms = (woke - next_tick).max(0.0) as f32;
            // After a long stall, start a fresh schedule instead of catching up.
            next_tick = (next_tick + TICK_MS).max(woke + TICK_MS / 2.0);
            if gate.state() != GateState::Running {
                continue;
            }
            if self.on_power(&mut sources) {
                next_tick = unbiased_ms() + TICK_MS;
                trim_at = Some(ticks + TRIM_AFTER_TICKS);
                continue;
            }
            if self.sensitive.expire(wall_ms()) {
                shutdown::wake();
            }

            ticks += 1;
            self.tick(&mut sources, woke, sampler_lag_ms, ticks);
            if !sources.pdh_open && sources.pdh_backoff.due() {
                self.open_pdh(&mut sources);
                sources.prime_pdh();
            }
            if trim_at == Some(ticks) {
                trim_at = None;
                release_idle_pages();
            }
        }
    }

    /// Waits `due` for the next tick on the timer, or with a plain sleep while there is none.
    /// The timer is created again on a growing wait; a pause or a stop ends the sleep at once.
    fn pace(&mut self, sources: &mut Sources, due: Duration, gate: &RunGate) {
        if sources.timer.is_none() && sources.timer_backoff.due() {
            match self.read(Source::Timer, sources.readings.open_timer()) {
                Ok(timer) => {
                    sources.timer = Some(timer);
                    sources.timer_backoff = Backoff::new();
                }
                Err(_) => sources.timer_backoff.failed(),
            }
        }
        if let Some(timer) = &sources.timer {
            let waited = timer.wait(due);
            if self.read(Source::Timer, waited).is_ok() {
                return;
            }
            sources.timer = None;
            sources.timer_backoff.failed();
        }
        gate.wait_while_running_for(due);
    }

    /// Tries to open the performance counters; a failure schedules the next attempt.
    fn open_pdh(&mut self, sources: &mut Sources) {
        let opened = self.read(Source::Pdh, sources.readings.open_pdh());
        sources.pdh_open = opened.is_ok();
        if opened.is_err() {
            sources.pdh_backoff.failed();
        }
    }

    fn tick(&mut self, sources: &mut Sources, woke: f64, sampler_lag_ms: f32, ticks: usize) {
        let ts_ms = wall_ms();
        if self.cleared.swap(false, Ordering::AcqRel) {
            self.forget(ts_ms);
        }
        self.blackbox.cut_gaps_to(ts_ms);
        self.watch_clock(ts_ms, woke);
        let sensitive = self.sensitive.is_on();
        self.blackbox.set_sensitive(sensitive);
        let mut probe_max_lag_ms: f32 = 0.0;
        let mut ticks_seen = Vec::new();
        for tick in self.probe_rx.try_iter() {
            probe_max_lag_ms = probe_max_lag_ms.max(tick.lag_ms);
            ticks_seen.push(tick);
            self.blackbox.push_probe(tick);
        }
        if sensitive {
            keep_hiccups(&mut lock(&self.hiccups), &ticks_seen, ts_ms);
        }
        // Each source fails alone: the rest of the tick runs whatever the others returned.
        let memory = self.read(Source::Memory, sources.readings.memory()).ok();
        let mut strain = Strain {
            mem_used_pct: memory.map_or(0.0, |m| used_pct(m.total_bytes, m.avail_bytes)),
            worst_lag_ms: probe_max_lag_ms.max(sampler_lag_ms),
            hardware_error: self.hardware_error,
            ..Strain::default()
        };
        let cpu = self
            .read(Source::Cpu, sources.readings.cpu())
            .ok()
            .flatten();
        let pdh = if sources.pdh_open {
            self.read(Source::Pdh, sources.readings.pdh())
                .ok()
                .flatten()
        } else {
            None
        };
        if let (Some(memory), Some(cpu), Some(pdh)) = (memory, cpu, pdh) {
            let sample = Sample {
                ts_ms,
                cpu_pct: cpu.cpu_pct,
                dpc_pct: cpu.dpc_pct,
                isr_pct: cpu.isr_pct,
                perf_limit_pct: pdh.perf_limit_pct,
                proc_perf_pct: pdh.proc_perf_pct,
                mem_total_bytes: memory.total_bytes,
                mem_avail_bytes: memory.avail_bytes,
                commit_bytes: memory.commit_bytes,
                commit_limit_bytes: memory.commit_limit_bytes,
                hard_faults_per_s: pdh.hard_faults_per_s,
                gpu_pct: pdh.gpu.as_ref().map(|gpu| gpu.system_pct),
                probe_max_lag_ms,
                sampler_lag_ms,
                cores: cpu.cores,
                disks: pdh.disks,
            };
            self.gpu_by_pid = pdh.gpu.map(|gpu| gpu.per_pid).unwrap_or_default();
            self.learn_machine(&sample);
            for disk in sample
                .disks
                .iter()
                .filter(|d| d.active_pct > 0.0 || d.bytes_per_s > 0.0)
            {
                self.disk_active_ms.insert(disk.name.clone(), ts_ms);
            }
            self.commit_limit_bytes = sample.commit_limit_bytes;
            {
                let mut readings = lock(&self.readings);
                readings.drop_while(|r| r.ts_ms < ts_ms - RING_MS);
                readings.push(Reading::of_sample(&sample));
            }
            self.blackbox.push_sample(sample.clone());
            if let Some(ring) = self.ring.as_mut() {
                ring.push_sample(&sample);
            }
            {
                let pressure = self.blackbox.open_pressure();
                strain.pressure_open = !pressure.is_empty();
                let mut now = lock(&self.now);
                now.sample = Some(sample.clone());
                now.pressure = pressure;
            }
            *lock(&self.pressure) = self.blackbox.recent_pressure();
            self.batcher.push_sample(sample);
            lock(&self.status).last_sample_ms = Some(ts_ms);
        }

        let snapshot = self
            .read(Source::Processes, sources.readings.processes())
            .ok();
        let snapshotted = snapshot.is_some();
        if let Some(snapshot) = snapshot {
            if self.snapshots.len() > TOP_EVERY {
                self.snapshots.pop_front();
            }
            self.snapshots.push_back((woke, snapshot));
        }
        if ticks.is_multiple_of(HANG_EVERY) {
            self.check_hangs(ts_ms);
        }
        if snapshotted
            && ticks.is_multiple_of(TOP_EVERY)
            && self.snapshots.len() > TOP_EVERY
            && let (Some((then, old)), Some((now, _))) =
                (self.snapshots.front(), self.snapshots.back())
        {
            // Processes that exited since `old` keep the CPU and I/O they used before going.
            let since: Vec<&[ProcessCounters]> = self
                .snapshots
                .iter()
                .skip(1)
                .map(|(_, s)| s.as_slice())
                .collect();
            let rates = process_rates(
                old,
                &last_seen(&since),
                (now - then).max(0.0) as u64,
                self.logical_cpus,
                &self.gpu_by_pid,
            );
            // The live screen ranks programs; detection and history keep each process.
            let mut all = programs(&rates);
            let heaviest_programs = heaviest(all.clone(), TOP_PER_METRIC);
            all.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
            all.truncate(PROGRAMS_LISTED);
            lock(&self.programs).push(ts_ms, &all);
            {
                let mut now = lock(&self.now);
                now.top = Some(ProcessTop {
                    ts_ms,
                    entries: heaviest_programs,
                });
                now.programs = Some(ProcessTop {
                    ts_ms,
                    entries: all,
                });
            }
            let top = ProcessTop {
                ts_ms,
                entries: heaviest(rates, TOP_PER_METRIC),
            };
            self.blackbox.push_top(top.clone());
            if let Some(ring) = self.ring.as_mut() {
                ring.push_top(&top);
            }
            self.batcher.push_processes(top);
        }

        if ticks.is_multiple_of(TOP_EVERY) {
            self.note_own_cpu();
        }
        self.take_events();
        self.settle_fall(ts_ms, false);
        if mem::take(&mut self.dumps_due) {
            self.check_dumps(ts_ms);
        }
        let outputs = self.blackbox.poll(ts_ms);
        self.keep(outputs, ts_ms);
        self.publish_suggestion();
        self.sync_ring(ts_ms, strain);
        if self.batcher.due(ts_ms) || self.events_due(ts_ms) {
            self.flush();
        }
        if prune_due(self.pruned_hour, ts_ms) {
            self.pruned_hour = Some(ts_ms.div_euclid(HOUR_MS));
            let retention = lock(&self.settings).retention;
            // A refused prune is not an error; the next wall hour tries again.
            if let Err(err) = self.store.prune(ts_ms, &retention) {
                self.report(&err);
            }
            if self.first_health_ms.is_none() {
                self.check_health(ts_ms);
            }
        }
        let scanned = self.health_due.swap(false, Ordering::AcqRel);
        if scanned || self.first_health_ms.is_some_and(|at| ts_ms >= at) {
            self.first_health_ms = None;
            self.check_health(ts_ms);
        }
    }

    /// Closes the timeline when the wall clock stepped by more than the unbiased clock did:
    /// everything open is written and the rings start over, so no stamp spans the step.
    fn watch_clock(&mut self, ts_ms: i64, woke: f64) {
        let Some(jump) = self.clock.observe(ts_ms, woke) else {
            return;
        };
        self.probe_rx.try_iter().for_each(drop);
        let outputs = self
            .blackbox
            .clock_changed(jump.from_wall_ms, jump.to_wall_ms);
        self.keep(outputs, ts_ms);
        self.flush_final();
        self.ring_sync = RingSync::new(ts_ms);
        lock(&self.readings).clear();
        lock(&self.programs).clear();
        lock(&self.hiccups).clear();
        self.publish_recent(ts_ms);
    }

    /// Lists the dumps Windows kept, without opening any.
    fn check_dumps(&mut self, ts_ms: i64) {
        if let Err(err) = take_inventory(&mut self.store, &DumpPlaces::of_this_pc(), ts_ms) {
            self.report(&err);
        }
    }

    /// Runs a health pass and lists the dumps: a minute after start, then every full hour.
    fn check_health(&mut self, ts_ms: i64) {
        self.check_dumps(ts_ms);
        let busy_disks = self
            .disk_active_ms
            .iter()
            .filter(|(_, at)| ts_ms - **at <= DISK_BUSY_MS)
            .map(|(name, _)| name.clone())
            .collect();
        let machine = self.blackbox.machine().clone();
        let now = PassNow {
            ts_ms,
            machine: &machine,
            busy_disks: &busy_disks,
            commit_limit_bytes: self.commit_limit_bytes,
        };
        match self.health.run(&mut self.store, &now) {
            Ok(outcome) => {
                self.critical_findings = outcome.critical;
                self.publish_recent(ts_ms);
                if outcome.asked_security_center {
                    release_idle_pages();
                }
            }
            Err(err) => self.report(&err),
        }
    }

    /// Names the processes with a window not responding, from the latest process snapshot.
    fn check_hangs(&mut self, ts_ms: i64) {
        let Some((_, latest)) = self.snapshots.back() else {
            return;
        };
        let names: HashMap<u32, &str> = latest.iter().map(|p| (p.pid, p.name.as_str())).collect();
        let running: HashSet<u32> = names.keys().copied().collect();
        let hung: Vec<HungWindow> = hung_pids(std::process::id())
            .into_iter()
            .map(|pid| HungWindow {
                pid,
                name: names.get(&pid).copied().unwrap_or_default().to_string(),
            })
            .collect();
        self.blackbox.push_hangs(ts_ms, &hung, &running);
        lock(&self.now).hung = hung;
    }

    /// Moves delivered events into the black box and the next batch, with their bookmark.
    fn take_events(&mut self) {
        let now = wall_ms();
        while let Ok(delivered) = self.events_rx.try_recv() {
            let event = &delivered.event;
            if event.meaning == EventMeaning::HardwareError && event.ts_ms >= self.started_ms {
                self.hardware_error = true;
            }
            if matches!(
                event.meaning,
                EventMeaning::BugCheck | EventMeaning::AppCrash
            ) {
                self.dumps_due = true;
            }
            if let Some(fall) = self.fall.as_mut() {
                fall.offer(event, now);
            }
            self.blackbox.push_event(delivered.event.clone());
            self.events.push(delivered.event);
            self.bookmark_xml = Some(delivered.bookmark_xml);
        }
    }

    /// Stores occurrences and episodes right away, those that failed before first; gaps wait for
    /// the batch.
    fn keep(&mut self, outputs: Vec<Output>, now: i64) {
        let again = mem::take(&mut self.unstored);
        for output in again.into_iter().chain(outputs) {
            let stored = match &output {
                Output::Occurrence(occurrence) => {
                    self.store.insert_occurrence(occurrence).map(|id| {
                        lock(&self.now).latest = Some(OccurrenceSummary {
                            id,
                            kind: occurrence.kind,
                            start_ms: occurrence.start_ms,
                            end_ms: occurrence.end_ms,
                            severity: occurrence.severity,
                            cause: occurrence.cause(),
                        });
                    })
                }
                Output::Episode(episode) => self.store.insert_episode(episode).map(drop),
                Output::Gap(gap) => {
                    self.gaps.push(*gap);
                    Ok(())
                }
            };
            if let Err(err) = stored {
                self.report(&err);
                self.hold_unstored(output);
            }
        }
        self.publish_recent(now);
    }

    /// Keeps an output that failed to store for the next try; past the cap the oldest is dropped
    /// and no longer counted.
    fn hold_unstored(&mut self, output: Output) {
        if matches!(output, Output::Gap(_)) {
            return;
        }
        self.unstored.push_back(output);
        while self.unstored.len() > OUTPUTS_KEPT
            && let Some(oldest) = self.unstored.pop_front()
        {
            self.blackbox.forget_notable(&oldest);
        }
    }

    /// Turns a fall Windows has spoken about into a post-mortem, and records the time away;
    /// `force` settles it with what is known so far.
    fn settle_fall(&mut self, now: i64, force: bool) {
        if !force && !self.fall.as_ref().is_some_and(|f| f.settled(now)) {
            return;
        }
        let Some(fall) = self.fall.take() else {
            return;
        };
        let reason = match fall.occurrence(&self.store) {
            Ok(Some(occurrence)) => {
                let severe = occurrence.severity == Severity::Severe;
                self.blackbox
                    .remember(occurrence.end_ms, Notable::Occurrence { severe });
                self.keep(vec![Output::Occurrence(Box::new(occurrence))], now);
                GapReason::Crashed
            }
            Ok(None) => GapReason::Stopped,
            Err(err) => {
                self.report(&err);
                GapReason::Stopped
            }
        };
        if let Some(start_ms) = self.held_gap.take() {
            self.blackbox.push_gap(Gap {
                start_ms,
                end_ms: self.started_ms.max(start_ms),
                reason,
            });
        }
    }

    /// Puts the black box on disk when its sync is due.
    fn sync_ring(&mut self, now: i64, strain: Strain) {
        if let Some(ring) = &self.ring
            && self.ring_sync.due(now, strain)
            && let Err(err) = ring.flush()
        {
            self.report(&err);
        }
    }

    /// Marks the black box closed cleanly, once everything is written.
    fn close_ring(&mut self) {
        if let Some(ring) = self.ring.as_mut()
            && let Err(err) = ring.close_cleanly()
        {
            self.report(&err);
        }
    }

    /// Drops what was watched before the history was cleared; the event bookmark stays.
    fn forget(&mut self, now: i64) {
        self.fall = None;
        self.held_gap = None;
        let _ = self.batcher.take_final();
        self.events.clear();
        self.gaps.clear();
        if self.bookmark_xml.is_none() {
            self.bookmark_xml = self
                .waiting
                .iter()
                .rev()
                .find_map(|b| b.bookmark_xml.clone());
        }
        self.waiting.clear();
        self.unstored.clear();
        self.blackbox = self.blackbox.emptied();
        lock(&self.readings).clear();
        lock(&self.pressure).clear();
        lock(&self.hiccups).clear();
        lock(&self.now).latest = None;
        self.publish_recent(now);
    }

    /// Hands the tray whether sensitive mode saw hitches worth a deep capture.
    fn publish_suggestion(&self) {
        let suggests = self.blackbox.suggests_capture();
        let mut status = lock(&self.status);
        if status.suggest_capture != suggests {
            status.suggest_capture = suggests;
            drop(status);
            shutdown::wake();
        }
    }

    /// Hands the tray the counts of the last 24 h, and wakes it when they changed.
    fn publish_recent(&self, now: i64) {
        let recent = RecentState {
            critical_findings: self.critical_findings,
            ..self.blackbox.recent(now)
        };
        let mut shown = lock(&self.recent);
        if *shown != recent {
            *shown = recent;
            drop(shown);
            shutdown::wake();
        }
    }

    /// Stores the machine facts once the disk counters name the disks.
    fn learn_machine(&mut self, sample: &Sample) {
        if sample.disks.is_empty() {
            return;
        }
        let known = &self.blackbox.machine().disks;
        if known.len() == sample.disks.len()
            && known
                .iter()
                .zip(&sample.disks)
                .all(|(k, d)| k.name == d.name)
        {
            return;
        }
        let machine = MachineInfo {
            logical_cpus: self.logical_cpus,
            mem_total_bytes: sample.mem_total_bytes,
            disks: sample
                .disks
                .iter()
                .map(|d| DiskInfo {
                    name: d.name.clone(),
                    media: physical_drive(&d.name).map_or(Media::Unknown, disk_media),
                })
                .collect(),
        };
        if let Err(err) = self.store.set_machine(&machine) {
            self.report(&err);
        }
        self.blackbox.set_machine(machine);
    }

    /// Handles pending suspend and resume notices; true when a resume restarted the sources.
    fn on_power(&mut self, sources: &mut Sources) -> bool {
        let mut resumed = false;
        while let Ok(event) = self.power_rx.try_recv() {
            match event {
                PowerEvent::Suspend => self.leave(GapReason::Suspended),
                PowerEvent::Resume => {
                    // Probe ticks and rates measured across the suspension are dropped.
                    self.probe_rx.try_iter().for_each(drop);
                    self.come_back(wall_ms());
                    self.clock.rebase(wall_ms(), unbiased_ms());
                    sources.reset();
                    sources.prime();
                    self.snapshots.clear();
                    resumed = true;
                }
            }
        }
        resumed
    }

    /// Closes what the black box holds and writes everything, before a pause, a suspend or a stop.
    /// Blocks while paused and ends a timed pause when its time comes; the wall clock is read
    /// again at least every minute, so a pause spent asleep still ends on time after waking.
    fn wait_out_pause(&self, gate: &RunGate) -> GateState {
        loop {
            let until = lock(&self.status).paused_until_ms;
            let left_ms = until.map_or(PAUSE_CHECK_MS, |end| {
                (end - wall_ms()).clamp(0, PAUSE_CHECK_MS)
            });
            if until.is_some() && left_ms == 0 {
                {
                    let mut status = lock(&self.status);
                    status.paused = false;
                    status.paused_until_ms = None;
                }
                gate.set(GateState::Running);
                shutdown::wake();
                return GateState::Running;
            }
            match gate.wait_while_paused_for(Duration::from_millis(left_ms as u64)) {
                GateState::Paused => continue,
                state => return state,
            }
        }
    }

    /// Updates the agent's own CPU share from the latest process snapshot.
    fn note_own_cpu(&mut self) {
        let own = std::process::id();
        let Some((woke, cpu_time_100ns)) = self.snapshots.back().and_then(|(woke, snapshot)| {
            let me = snapshot.iter().find(|p| p.pid == own)?;
            Some((*woke, me.cpu_time_100ns))
        }) else {
            return;
        };
        if let Some(pct) = self.own_cpu.push(woke, cpu_time_100ns, self.logical_cpus) {
            lock(&self.status).cpu_pct = Some(pct);
        }
    }

    fn leave(&mut self, reason: GapReason) {
        let now = wall_ms();
        self.own_cpu.clear();
        lock(&self.status).cpu_pct = None;
        self.take_events();
        let outputs = self.blackbox.drain(now);
        self.keep(outputs, now);
        lock(&self.now).pressure.clear();
        *lock(&self.pressure) = self.blackbox.recent_pressure();
        self.flush_final();
        if let Some(ring) = &self.ring
            && let Err(err) = ring.flush()
        {
            self.report(&err);
        }
        if self.away_since.is_none() {
            self.away_since = Some((now, reason));
        }
    }

    /// Records the time spent away as a gap; a second resume notice finds nothing to record.
    fn come_back(&mut self, now: i64) {
        if let Some((start_ms, reason)) = self.away_since.take() {
            self.blackbox.push_gap(Gap {
                start_ms,
                end_ms: now.max(start_ms),
                reason,
            });
        }
    }

    /// True when events, gaps or a failed batch have waited a whole interval for samples that
    /// did not come.
    fn events_due(&self, now_ms: i64) -> bool {
        (!self.events.is_empty() || !self.gaps.is_empty() || !self.waiting.is_empty())
            && now_ms.saturating_sub(self.last_write_ms) >= self.flush_ms
    }

    /// Writes whatever is buffered except the minute still open.
    fn flush(&mut self) {
        let segments = self.batcher.take();
        self.write(segments);
    }

    /// Writes everything buffered, the open minute included.
    fn flush_final(&mut self) {
        let segments = self.batcher.take_final();
        self.write(segments);
    }

    /// Writes segments, events with their bookmark, and gaps in one transaction; a batch that
    /// fails waits for the next flush.
    fn write(&mut self, segments: Vec<Segment>) {
        self.last_write_ms = wall_ms();
        let batch = Batch {
            segments,
            events: mem::take(&mut self.events),
            bookmark_xml: self.bookmark_xml.take(),
            gaps: mem::take(&mut self.gaps),
        };
        if !batch.is_empty() {
            self.waiting.push_back(batch);
        }
        self.send_waiting();
    }

    /// Writes the waiting batches oldest first and stops at the first failure; beyond the cap
    /// the oldest loses its samples.
    fn send_waiting(&mut self) {
        while let Some(batch) = self.waiting.pop_front() {
            let Err(err) = self.store.append_batch(&batch) else {
                continue;
            };
            self.report(&err);
            if matches!(err, StoreError::Sqlite(_)) {
                self.waiting.push_front(batch);
                break;
            }
            if let Some(kept) = without_unstorable(batch) {
                self.waiting.push_front(kept);
            }
        }
        while self.waiting.len() > BATCHES_KEPT {
            self.shed_oldest();
        }
    }

    /// Drops the samples of the oldest waiting batch; its events, gaps and bookmark move into
    /// the next one.
    fn shed_oldest(&mut self) {
        let Some(oldest) = self.waiting.pop_front() else {
            return;
        };
        if let Some(next) = self.waiting.front_mut() {
            next.events.splice(0..0, oldest.events);
            next.gaps.splice(0..0, oldest.gaps);
            next.bookmark_xml = next.bookmark_xml.take().or(oldest.bookmark_xml);
        }
    }

    /// Notes how a source read, tells the status about a new error, and hands the result back.
    fn read<T>(
        &mut self,
        source: Source,
        result: Result<T, CollectError>,
    ) -> Result<T, CollectError> {
        if let Some(text) = self.source_health.record(source, wall_ms(), &result) {
            self.report(&text);
        }
        result
    }

    fn report(&self, err: &dyn std::fmt::Display) {
        lock(&self.status).last_error = Some(err.to_string());
    }
}

/// What of a batch can still be stored after a value in it had no stored code: its samples and
/// bookmark; `None` when the batch already holds nothing else.
fn without_unstorable(batch: Batch) -> Option<Batch> {
    if batch.events.is_empty() && batch.gaps.is_empty() {
        return None;
    }
    let kept = Batch {
        segments: batch.segments,
        bookmark_xml: batch.bookmark_xml,
        ..Batch::default()
    };
    (!kept.is_empty()).then_some(kept)
}

/// Adds the late wakes among `ticks` to `kept`, then drops what is old or beyond the cap.
fn keep_hiccups(kept: &mut VecDeque<ProbeHiccup>, ticks: &[ProbeTick], now_ms: i64) {
    kept.extend(
        ticks
            .iter()
            .filter(|tick| tick.lag_ms >= HICCUP_MS.start)
            .map(|tick| ProbeHiccup {
                ts_ms: tick.wall_ms,
                lag_ms: tick.lag_ms,
            }),
    );
    while kept
        .front()
        .is_some_and(|h| now_ms - h.ts_ms > HICCUPS_KEPT_MS)
    {
        kept.pop_front();
    }
    while kept.len() > HICCUPS_KEPT {
        kept.pop_front();
    }
}

fn used_pct(total_bytes: u64, avail_bytes: u64) -> f32 {
    if total_bytes == 0 {
        return 0.0;
    }
    (total_bytes.saturating_sub(avail_bytes) as f64 * 100.0 / total_bytes as f64) as f32
}

/// Hands back the pages only startup touched (PDH name tables, DLL setup); the rest fault back in.
fn release_idle_pages() {
    // SAFETY: (usize::MAX, usize::MAX) asks Windows to trim this process's working set.
    let _ = unsafe { SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX) };
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Instant;

    use vestigium_collect::ProbeConfig;
    use vestigium_core::{
        Cause, Channel, CrashKind, DEFAULT_PROBE_PERIOD, EventMeaning, OccurrenceKind,
        ProcessSample, SENSITIVE_FOR, Sample, Trigger,
    };
    use vestigium_store::{Series, decode, encode};

    use super::*;
    use crate::readings::{Waiter, windows_source_set};
    use vestigium_collect::{CpuReading, MemoryInfo, PdhReading};
    use vestigium_core::DiskLoad;

    struct Running {
        gate: Arc<RunGate>,
        handle: SamplerHandle,
        status: Arc<Mutex<AgentStatus>>,
        now: Arc<Mutex<NowFrame>>,
        probe_tx: mpsc::Sender<ProbeTick>,
        power_tx: mpsc::Sender<PowerEvent>,
        events_tx: mpsc::Sender<Delivered>,
    }

    /// What a test keeps of a sampler's wiring: the gate, the shared state and the senders.
    struct Ties {
        gate: Arc<RunGate>,
        status: Arc<Mutex<AgentStatus>>,
        now: Arc<Mutex<NowFrame>>,
        probe_tx: mpsc::Sender<ProbeTick>,
        power_tx: mpsc::Sender<PowerEvent>,
        events_tx: mpsc::Sender<Delivered>,
    }

    /// Occurrences close one second after their trigger, so tests do not wait half a minute.
    const QUICK: Timing = Timing {
        before_ms: 5_000,
        after_ms: 1_000,
        merge_ms: 1_000,
        max_ms: 60_000,
    };

    fn start(dir: &std::path::Path, flush: Duration) -> Running {
        start_with(dir, flush, windows_source_set())
    }

    fn start_with(dir: &std::path::Path, flush: Duration, source_set: SourceSet) -> Running {
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let (wiring, ties) = wire(dir, flush, source_set);
        Running {
            handle: run_sampler(store, wiring).unwrap(),
            gate: ties.gate,
            status: ties.status,
            now: ties.now,
            probe_tx: ties.probe_tx,
            power_tx: ties.power_tx,
            events_tx: ties.events_tx,
        }
    }

    /// The wiring of a sampler and the handles a test uses to drive it.
    fn wire(dir: &std::path::Path, flush: Duration, source_set: SourceSet) -> (Wiring, Ties) {
        let gate = RunGate::new();
        let (probe_tx, probe_rx) = mpsc::channel();
        let (power_tx, power_rx) = mpsc::channel();
        let (events_tx, events_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(AgentStatus::default()));
        let sensitive = SensitiveMode::new(ProbeConfig::new(DEFAULT_PROBE_PERIOD), SENSITIVE_FOR);
        let now = Arc::new(Mutex::new(NowFrame::default()));
        let wiring = Wiring {
            flush,
            timing: QUICK,
            gate: gate.clone(),
            probe_rx,
            power_rx,
            events_rx,
            subscription: None,
            sensitive,
            status: status.clone(),
            recent: Arc::new(Mutex::new(RecentState::default())),
            now: now.clone(),
            readings: Arc::new(Mutex::new(Ring::with_capacity(660))),
            programs: Arc::new(Mutex::new(ProgramRing::new(RING_MS))),
            pressure: Arc::new(Mutex::new(Vec::new())),
            hiccups: Arc::new(Mutex::new(VecDeque::new())),
            settings: Arc::new(Mutex::new(Settings::default())),
            cleared: Arc::new(AtomicBool::new(false)),
            health_due: Arc::new(AtomicBool::new(false)),
            ring_path: Some(dir.join("blackbox.ring")),
            source_set,
        };
        let ties = Ties {
            gate,
            status,
            now,
            probe_tx,
            power_tx,
            events_tx,
        };
        (wiring, ties)
    }

    fn stop(running: &Running) {
        running.gate.set(GateState::Stopping);
        assert!(
            running.handle.wait(Duration::from_secs(3)),
            "sampler did not stop"
        );
    }

    fn history(dir: &std::path::Path) -> Store {
        Store::open(&dir.join("vestigium.db")).unwrap()
    }

    fn samples(dir: &std::path::Path) -> (usize, usize, Option<i64>) {
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let segments = store.segments(Series::Samples, i64::MIN, i64::MAX).unwrap();
        let count = segments
            .iter()
            .map(|s| decode::<Sample>(&s.data).unwrap().len())
            .sum();
        (segments.len(), count, segments.last().map(|s| s.end_ms))
    }

    /// Polls `done` until it holds or `timeout` passes; CI runners are slow and noisy.
    fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if done() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        done()
    }

    const PATIENCE: Duration = Duration::from_secs(15);
    const NEVER_DUE: Duration = Duration::from_secs(3600);

    fn sampled(status: &Mutex<AgentStatus>) -> bool {
        lock(status).last_sample_ms.is_some()
    }

    /// Sleeps for the wait, as the high-resolution timer would.
    struct Sleeper;

    impl Waiter for Sleeper {
        fn wait(&self, due: Duration) -> Result<(), CollectError> {
            thread::sleep(due);
            Ok(())
        }
    }

    /// Readings of a quiet machine, every source working.
    struct Quiet;

    impl Readings for Quiet {
        fn memory(&mut self) -> Result<MemoryInfo, CollectError> {
            Ok(MemoryInfo {
                total_bytes: 16 << 30,
                avail_bytes: 8 << 30,
                commit_bytes: 10 << 30,
                commit_limit_bytes: 24 << 30,
            })
        }

        fn cpu(&mut self) -> Result<Option<CpuReading>, CollectError> {
            Ok(Some(CpuReading {
                cpu_pct: 10.0,
                dpc_pct: 0.0,
                isr_pct: 0.0,
                cores: Vec::new(),
            }))
        }

        fn pdh(&mut self) -> Result<Option<PdhReading>, CollectError> {
            Ok(Some(PdhReading {
                perf_limit_pct: 100.0,
                proc_perf_pct: 100.0,
                hard_faults_per_s: 0.0,
                disks: Vec::new(),
                gpu: None,
            }))
        }

        fn processes(&mut self) -> Result<Vec<ProcessCounters>, CollectError> {
            Ok(Vec::new())
        }

        fn reset(&mut self) {}

        fn open_pdh(&mut self) -> Result<(), CollectError> {
            Ok(())
        }

        fn open_timer(&mut self) -> Result<Box<dyn Waiter>, CollectError> {
            Ok(Box::new(Sleeper))
        }
    }

    /// What a `Scripted` set does, shared with the test that watches it.
    #[derive(Default)]
    struct Script {
        /// Attempts to open the counters that still fail.
        pdh_open_failures: u32,
        pdh_open_attempts: u32,
        /// Attempts to create the timer that still fail.
        timer_open_failures: u32,
        /// Sources whose every read fails.
        failing: Vec<Source>,
        /// The counters name one disk while this is false, and none once it is true.
        disks_gone: bool,
    }

    impl Script {
        fn fails(&self, source: Source) -> bool {
            self.failing.contains(&source)
        }
    }

    /// A quiet machine whose counters and timer open as the script says.
    struct Scripted {
        script: Arc<Mutex<Script>>,
        pdh_open: bool,
        quiet: Quiet,
    }

    fn scripted(script: &Arc<Mutex<Script>>) -> SourceSet {
        let script = script.clone();
        Box::new(move || {
            Box::new(Scripted {
                script,
                pdh_open: false,
                quiet: Quiet,
            })
        })
    }

    fn refused(call: &'static str) -> CollectError {
        CollectError::Pdh {
            call,
            status: 0xC000_0BB8,
        }
    }

    impl Readings for Scripted {
        fn memory(&mut self) -> Result<MemoryInfo, CollectError> {
            if lock(&self.script).fails(Source::Memory) {
                return Err(refused("GlobalMemoryStatusEx"));
            }
            self.quiet.memory()
        }

        fn cpu(&mut self) -> Result<Option<CpuReading>, CollectError> {
            if lock(&self.script).fails(Source::Cpu) {
                return Err(refused("NtQuerySystemInformation"));
            }
            self.quiet.cpu()
        }

        fn pdh(&mut self) -> Result<Option<PdhReading>, CollectError> {
            let script = lock(&self.script);
            if !self.pdh_open {
                return Err(refused("read before the counters opened"));
            }
            if script.fails(Source::Pdh) {
                return Err(refused("PdhCollectQueryData"));
            }
            let mut reading = self.quiet.pdh()?;
            if let Some(reading) = reading.as_mut()
                && !script.disks_gone
            {
                reading.disks.push(DiskLoad {
                    name: "9 Z:".to_string(),
                    active_pct: 1.0,
                    latency_ms: 1.0,
                    queue_len: 0.0,
                    bytes_per_s: 0.0,
                });
            }
            Ok(reading)
        }

        fn processes(&mut self) -> Result<Vec<ProcessCounters>, CollectError> {
            if lock(&self.script).fails(Source::Processes) {
                return Err(refused("NtQuerySystemInformation"));
            }
            self.quiet.processes()
        }

        fn reset(&mut self) {}

        fn open_pdh(&mut self) -> Result<(), CollectError> {
            let mut script = lock(&self.script);
            script.pdh_open_attempts += 1;
            if script.pdh_open_failures > 0 {
                script.pdh_open_failures -= 1;
                return Err(refused("PdhAddEnglishCounterW"));
            }
            self.pdh_open = true;
            Ok(())
        }

        fn open_timer(&mut self) -> Result<Box<dyn Waiter>, CollectError> {
            let mut script = lock(&self.script);
            if script.timer_open_failures > 0 {
                script.timer_open_failures -= 1;
                return Err(refused("CreateWaitableTimerExW"));
            }
            Ok(Box::new(Sleeper))
        }
    }

    #[test]
    fn pdh_that_opens_late_starts_sampling_without_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script {
            pdh_open_failures: 2,
            ..Script::default()
        }));
        let running = start_with(dir.path(), NEVER_DUE, scripted(&script));

        thread::sleep(Duration::from_millis(500));
        assert!(
            !sampled(&running.status),
            "sampled before the counters opened"
        );
        let sampled_later = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_later, "the counters opened but nothing was sampled");
        assert_eq!(lock(&script).pdh_open_attempts, 3);
        assert!(lock(&running.status).last_error.is_some());
    }

    #[test]
    fn a_failing_open_still_turns_a_kernel_power_41_into_an_occurrence() {
        let dir = tempfile::tempdir().unwrap();
        let last = wall_ms() - 10 * 60_000;
        {
            let (mut ring, _) = BlackboxRing::open(&dir.path().join("blackbox.ring")).unwrap();
            for i in 0..30 {
                ring.push_sample(&quiet(last - 29_000 + i * 1_000));
            }
            // Dropped without a clean close, as a fall leaves it.
        }
        let script = Arc::new(Mutex::new(Script {
            pdh_open_failures: u32::MAX,
            ..Script::default()
        }));

        let running = start_with(dir.path(), Duration::from_secs(1), scripted(&script));
        let boot = wall_ms() - 60_000;
        running
            .events_tx
            .send(unexpected_shutdown(boot, last + 2_000))
            .unwrap();
        running.events_tx.send(power_loss(boot + 1_000)).unwrap();
        let found = wait_until(PATIENCE, || {
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|o| o.kind == OccurrenceKind::PostMortem)
        });
        stop(&running);

        assert!(
            found,
            "the fall left no post-mortem while the counters were down"
        );
        assert_eq!(lock(&running.status).last_sample_ms, None);
        assert!(lock(&script).pdh_open_attempts > 1, "no second attempt");
    }

    #[test]
    fn a_source_that_always_fails_still_stores_a_freeze() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script {
            failing: vec![Source::Memory, Source::Cpu, Source::Pdh, Source::Processes],
            ..Script::default()
        }));
        let running = start_with(dir.path(), Duration::from_secs(1), scripted(&script));

        let tick = ProbeTick {
            wall_ms: wall_ms(),
            lag_ms: 1_500.0,
        };
        running.probe_tx.send(tick).unwrap();
        let event = WinEvent {
            ts_ms: wall_ms(),
            channel: Channel::System,
            provider: "disk".to_string(),
            id: 153,
            record_id: 78,
            level: 3,
            meaning: EventMeaning::DiskRetry,
            data_xml: String::new(),
        };
        running
            .events_tx
            .send(Delivered {
                event: event.clone(),
                bookmark_xml: "<BookmarkList/>".to_string(),
            })
            .unwrap();
        let stored = wait_until(PATIENCE, || {
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|o| o.kind == OccurrenceKind::Freeze && o.start_ms == tick.wall_ms - 1_500)
                && history(dir.path()).events(i64::MIN, i64::MAX).unwrap() == vec![event.clone()]
        });
        stop(&running);

        assert!(stored, "the freeze or the event was not stored");
        assert_eq!(lock(&running.status).last_sample_ms, None);
        let (_, count, _) = samples(dir.path());
        assert!(count <= 300);
    }

    #[test]
    fn a_failing_process_list_does_not_stop_the_samples() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script {
            failing: vec![Source::Processes],
            ..Script::default()
        }));
        let running = start_with(dir.path(), NEVER_DUE, scripted(&script));

        let sampled_once = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_once, "no sample while the process list failed");
        assert!(lock(&running.now).top.is_none());
        assert!(lock(&running.status).last_error.is_some());
    }

    #[test]
    fn a_failing_source_does_not_erase_the_machine_disks() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script::default()));
        let running = start_with(dir.path(), NEVER_DUE, scripted(&script));
        let learned = wait_until(PATIENCE, || {
            history(dir.path())
                .machine()
                .unwrap()
                .is_some_and(|m| m.disks.len() == 1)
        });
        let before = lock(&running.status).last_sample_ms;

        lock(&script).disks_gone = true;
        let sampled_after = wait_until(PATIENCE, || {
            lock(&running.status).last_sample_ms > before.map(|ms| ms + 2_000)
        });
        stop(&running);

        assert!(learned, "the disk was never learned");
        assert!(sampled_after, "no sample after the disks vanished");
        let machine = history(dir.path()).machine().unwrap().unwrap();
        assert_eq!(machine.disks.len(), 1);
    }

    #[test]
    fn a_timer_that_will_not_open_still_ticks() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script {
            timer_open_failures: u32::MAX,
            ..Script::default()
        }));
        let running = start_with(dir.path(), NEVER_DUE, scripted(&script));

        let sampled_once = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_once, "no tick without the timer");
        assert!(lock(&running.status).last_error.is_some());
    }

    #[test]
    fn a_sampler_runs_on_a_replaced_source_set() {
        let dir = tempfile::tempdir().unwrap();
        let running = start_with(dir.path(), NEVER_DUE, Box::new(|| Box::new(Quiet)));

        let sampled_once = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_once, "no sample from the replaced sources");
        let (_, count, _) = samples(dir.path());
        assert!(count >= 1);
        assert_eq!(lock(&running.status).last_error, None);
    }

    #[test]
    fn sampler_writes_a_batch_once_it_is_due() {
        let dir = tempfile::tempdir().unwrap();
        let Running {
            gate,
            handle,
            status,
            ..
        } = start(dir.path(), Duration::from_secs(1));

        let written = wait_until(PATIENCE, || samples(dir.path()).0 >= 1);
        gate.set(GateState::Stopping);
        assert!(handle.wait(Duration::from_secs(3)), "sampler did not stop");

        assert!(written, "no batch was written while running");
        assert_eq!(lock(&status).last_error, None);
    }

    #[test]
    fn sampler_ranks_programs_for_the_live_screen() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), NEVER_DUE);

        let ranked = wait_until(PATIENCE, || lock(&running.now).top.is_some());
        let (top, programs) = {
            let now = lock(&running.now);
            (now.top.clone(), now.programs.clone())
        };
        stop(&running);

        assert!(ranked, "no ranking within the patience");
        let mut names: Vec<String> = top.unwrap().entries.into_iter().map(|p| p.name).collect();
        let listed = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), listed, "each program once");
        let all = programs
            .expect("the full list comes with the ranking")
            .entries;
        assert!(all.len() >= listed && all.len() <= PROGRAMS_LISTED);
        assert!(
            all.windows(2).all(|w| w[0].cpu_pct >= w[1].cpu_pct),
            "busiest first"
        );
    }

    #[test]
    fn sampler_writes_what_it_holds_when_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let Running {
            gate,
            handle,
            status,
            ..
        } = start(dir.path(), NEVER_DUE);
        assert!(wait_until(PATIENCE, || sampled(&status)), "no sample taken");
        assert_eq!(samples(dir.path()).0, 0, "nothing is due within the hour");

        gate.set(GateState::Stopping);
        assert!(handle.wait(Duration::from_secs(3)), "sampler did not stop");

        let (segments, count, last_end) = samples(dir.path());
        assert_eq!(segments, 1);
        assert!(count >= 1);
        assert_eq!(last_end, lock(&status).last_sample_ms);
    }

    #[test]
    fn sampler_flushes_on_suspend_and_resets_on_resume() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), NEVER_DUE);
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        running.power_tx.send(PowerEvent::Suspend).unwrap();
        assert!(
            wait_until(PATIENCE, || samples(dir.path()).0 == 1),
            "suspending did not write the buffered batch"
        );
        let resumed_at = wall_ms();
        running.power_tx.send(PowerEvent::Resume).unwrap();
        let sampled_again = wait_until(PATIENCE, || {
            lock(&running.status)
                .last_sample_ms
                .is_some_and(|ms| ms > resumed_at)
        });
        running.gate.set(GateState::Stopping);
        assert!(
            running.handle.wait(Duration::from_secs(3)),
            "sampler did not stop"
        );

        assert!(sampled_again, "no sample after resuming");
        assert_eq!(lock(&running.status).last_error, None);
    }

    #[test]
    fn paused_sampler_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let Running {
            gate,
            handle,
            status,
            ..
        } = start(dir.path(), NEVER_DUE);
        assert!(wait_until(PATIENCE, || sampled(&status)), "no sample taken");

        gate.set(GateState::Paused);
        assert!(
            wait_until(PATIENCE, || samples(dir.path()).0 == 1),
            "pausing did not write the buffered batch"
        );
        let before = samples(dir.path()).1;
        thread::sleep(Duration::from_secs(3));
        let after_pause = samples(dir.path()).1;
        gate.set(GateState::Stopping);
        assert!(handle.wait(Duration::from_secs(3)), "sampler did not stop");

        assert!(before >= 1);
        assert_eq!(after_pause, before);
        assert_eq!(
            samples(dir.path()).1,
            before,
            "stopping while paused adds nothing"
        );
    }

    #[test]
    fn sampler_stores_an_occurrence_from_a_late_tick() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), Duration::from_secs(1));
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        let tick = ProbeTick {
            wall_ms: wall_ms(),
            lag_ms: 1_500.0,
        };
        running.probe_tx.send(tick).unwrap();
        let stored = wait_until(PATIENCE, || {
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|o| o.kind == OccurrenceKind::Freeze && o.start_ms == tick.wall_ms - 1_500)
        });
        stop(&running);

        assert!(stored, "the late tick left no occurrence");
        assert_eq!(lock(&running.status).last_error, None);
    }

    #[test]
    fn stopped_gap_is_recorded_at_start() {
        let dir = tempfile::tempdir().unwrap();
        let last_end = wall_ms() - 3_600_000;
        history(dir.path())
            .append(&[Segment {
                series: Series::Samples,
                start_ms: last_end - 1_000,
                end_ms: last_end,
                data: encode::<Sample>(&[]),
            }])
            .unwrap();

        let running = start(dir.path(), Duration::from_secs(1));
        let recorded = wait_until(PATIENCE, || {
            history(dir.path())
                .gaps(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|g| g.reason == GapReason::Stopped && g.start_ms == last_end)
        });
        stop(&running);

        assert!(recorded, "no gap for the time the agent was stopped");
    }

    fn delivered(
        ts_ms: i64,
        provider: &str,
        id: u16,
        meaning: EventMeaning,
        data_xml: String,
    ) -> Delivered {
        Delivered {
            event: WinEvent {
                ts_ms,
                channel: Channel::System,
                provider: provider.to_string(),
                id,
                record_id: ts_ms as u64,
                level: 1,
                meaning,
                data_xml,
            },
            bookmark_xml: "<BookmarkList/>".to_string(),
        }
    }

    /// The Kernel-Power 41 of a blue screen with stop code 0x50, as the next boot logs it.
    fn power_loss(ts_ms: i64) -> Delivered {
        let data = "<EventData><Data Name='BugcheckCode'>80</Data>\
            <Data Name='SleepInProgress'>0</Data><Data Name='PowerButtonTimestamp'>0</Data></EventData>";
        delivered(
            ts_ms,
            "Microsoft-Windows-Kernel-Power",
            41,
            EventMeaning::PowerLoss,
            data.to_string(),
        )
    }

    /// The EventLog 6008 that dates a fall at `down_ms`, in UTC.
    fn unexpected_shutdown(ts_ms: i64, down_ms: i64) -> Delivered {
        let utc = systemtime(down_ms);
        let hex: String = utc
            .iter()
            .chain(utc.iter())
            .flat_map(|v| v.to_le_bytes())
            .map(|b| format!("{b:02X}"))
            .collect();
        let data = format!("<EventData><Binary>{hex}</Binary></EventData>");
        delivered(
            ts_ms,
            "EventLog",
            6008,
            EventMeaning::UnexpectedShutdown,
            data,
        )
    }

    /// A Windows SYSTEMTIME for a Unix time: year, month, weekday, day, hour, minute, second, ms.
    fn systemtime(ms: i64) -> [u16; 8] {
        let (days, rest) = (ms.div_euclid(DAY_MS), ms.rem_euclid(DAY_MS));
        // Civil date from days since 1970-01-01, after Howard Hinnant's algorithm.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        let parts = [
            year,
            month,
            0,
            day,
            rest / HOUR_MS,
            rest / 60_000 % 60,
            rest / 1000 % 60,
            rest % 1000,
        ];
        parts.map(|v| v as u16)
    }

    fn quiet(ts_ms: i64) -> Sample {
        Sample {
            ts_ms,
            cpu_pct: 20.0,
            dpc_pct: 0.0,
            isr_pct: 0.0,
            perf_limit_pct: 100.0,
            proc_perf_pct: 100.0,
            mem_total_bytes: 16 << 30,
            mem_avail_bytes: 1 << 30,
            commit_bytes: 20 << 30,
            commit_limit_bytes: 24 << 30,
            hard_faults_per_s: 0.0,
            gpu_pct: None,
            probe_max_lag_ms: 1.0,
            sampler_lag_ms: 0.0,
            cores: Vec::new(),
            disks: Vec::new(),
        }
    }

    #[test]
    fn clean_exit_is_not_a_crash() {
        let dir = tempfile::tempdir().unwrap();
        let first = start(dir.path(), Duration::from_secs(1));
        assert!(wait_until(PATIENCE, || sampled(&first.status)));
        stop(&first);

        let second = start(dir.path(), Duration::from_secs(1));
        let now = wall_ms();
        second
            .events_tx
            .send(unexpected_shutdown(now, now - 1_000))
            .unwrap();
        second.events_tx.send(power_loss(now + 1)).unwrap();
        let taken = wait_until(PATIENCE, || {
            history(dir.path())
                .events(i64::MIN, i64::MAX)
                .unwrap()
                .len()
                == 2
        });
        stop(&second);

        assert!(taken, "the events never reached the history");
        let store = history(dir.path());
        let occurrences = store.occurrences(i64::MIN, i64::MAX).unwrap();
        assert!(
            occurrences
                .iter()
                .all(|o| o.kind != OccurrenceKind::PostMortem)
        );
        let gaps = store.gaps(i64::MIN, i64::MAX).unwrap();
        assert!(gaps.iter().all(|g| g.reason != GapReason::Crashed));
    }

    #[test]
    fn crash_becomes_a_postmortem_occurrence() {
        let dir = tempfile::tempdir().unwrap();
        let last = wall_ms() - 10 * 60_000;
        {
            let (mut ring, _) = BlackboxRing::open(&dir.path().join("blackbox.ring")).unwrap();
            for i in 0..30 {
                ring.push_sample(&quiet(last - 29_000 + i * 1_000));
            }
            // Dropped without a clean close, as a fall leaves it.
        }

        let running = start(dir.path(), Duration::from_secs(1));
        let boot = wall_ms() - 60_000;
        running
            .events_tx
            .send(unexpected_shutdown(boot, last + 2_000))
            .unwrap();
        running.events_tx.send(power_loss(boot + 1_000)).unwrap();
        let found = wait_until(PATIENCE, || {
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|o| o.kind == OccurrenceKind::PostMortem)
        });
        stop(&running);

        assert!(found, "the fall left no post-mortem");
        let store = history(dir.path());
        let summary = store
            .occurrences(i64::MIN, i64::MAX)
            .unwrap()
            .into_iter()
            .find(|o| o.kind == OccurrenceKind::PostMortem)
            .unwrap();
        let occurrence = store.occurrence(summary.id).unwrap().unwrap();
        assert!(matches!(
            occurrence.triggers[..],
            [Trigger::Crash {
                last_ms,
                down_ms,
                kind: CrashKind::BlueScreen { code: 0x50, .. },
            }] if last_ms == last && down_ms == last + 2_000
        ));
        assert_eq!(occurrence.window.samples.len(), 30);
        let gaps = store.gaps(i64::MIN, i64::MAX).unwrap();
        assert!(
            gaps.iter()
                .any(|g| g.reason == GapReason::Crashed && g.start_ms == last),
            "the time away was not recorded as a fall: {gaps:?}"
        );
    }

    #[test]
    fn a_disk_event_before_the_last_second_reaches_the_postmortem() {
        let dir = tempfile::tempdir().unwrap();
        let last = wall_ms() - 10 * 60_000;
        {
            let (mut ring, _) = BlackboxRing::open(&dir.path().join("blackbox.ring")).unwrap();
            for i in 0..30 {
                ring.push_sample(&quiet(last - 29_000 + i * 1_000));
            }
            // Dropped without a clean close, as a fall leaves it.
        }

        let running = start(dir.path(), Duration::from_secs(1));
        let boot = wall_ms() - 60_000;
        // The old bookmark delivers what was logged before the stop first, then the next boot's events.
        running
            .events_tx
            .send(delivered(
                last - 30_000,
                "disk",
                153,
                EventMeaning::DiskRetry,
                String::new(),
            ))
            .unwrap();
        running
            .events_tx
            .send(unexpected_shutdown(boot, last + 2_000))
            .unwrap();
        let frozen = "<EventData><Data Name='BugcheckCode'>0</Data>            <Data Name='SleepInProgress'>0</Data><Data Name='PowerButtonTimestamp'>0</Data></EventData>";
        running
            .events_tx
            .send(delivered(
                boot + 1_000,
                "Microsoft-Windows-Kernel-Power",
                41,
                EventMeaning::PowerLoss,
                frozen.to_string(),
            ))
            .unwrap();
        let found = wait_until(PATIENCE, || {
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|o| o.kind == OccurrenceKind::PostMortem)
        });
        stop(&running);

        assert!(found, "the fall left no post-mortem");
        let store = history(dir.path());
        let summary = store
            .occurrences(i64::MIN, i64::MAX)
            .unwrap()
            .into_iter()
            .find(|o| o.kind == OccurrenceKind::PostMortem)
            .unwrap();
        let occurrence = store.occurrence(summary.id).unwrap().unwrap();
        assert!(
            occurrence
                .window
                .events
                .iter()
                .any(|e| e.meaning == EventMeaning::DiskRetry && e.ts_ms == last - 30_000),
            "the disk event is missing from the window: {:?}",
            occurrence.window.events
        );
        assert!(
            occurrence
                .suspects
                .iter()
                .any(|s| matches!(s.cause, Cause::Storage { .. })),
            "no storage suspect: {:?}",
            occurrence.suspects
        );
    }

    #[test]
    fn events_and_bookmark_are_stored_together() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), Duration::from_secs(1));
        let event = WinEvent {
            ts_ms: wall_ms(),
            channel: Channel::System,
            provider: "disk".to_string(),
            id: 153,
            record_id: 77,
            level: 3,
            meaning: EventMeaning::DiskRetry,
            data_xml: String::new(),
        };
        running
            .events_tx
            .send(Delivered {
                event: event.clone(),
                bookmark_xml: "<BookmarkList/>".to_string(),
            })
            .unwrap();

        let stored = wait_until(PATIENCE, || {
            let store = history(dir.path());
            store.events(i64::MIN, i64::MAX).unwrap() == vec![event.clone()]
                && store.bookmark_xml().unwrap().as_deref() == Some("<BookmarkList/>")
        });
        stop(&running);

        assert!(stored, "the event and its bookmark were not stored");
    }

    fn disk_event(record_id: u64) -> WinEvent {
        WinEvent {
            ts_ms: wall_ms(),
            channel: Channel::System,
            provider: "disk".to_string(),
            id: 153,
            record_id,
            level: 3,
            meaning: EventMeaning::DiskRetry,
            data_xml: String::new(),
        }
    }

    /// A sampler driven by hand over a history file in `dir`.
    fn manual_sampler(dir: &std::path::Path) -> Sampler {
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let (wiring, _) = wire(dir, Duration::from_secs(300), windows_source_set());
        Sampler::new(store, wiring)
    }

    #[test]
    fn a_clock_jump_writes_what_is_open_and_empties_the_rings() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let ts = wall_ms();
        sampler.clock = ClockWatch::new(ts, 0.0);
        sampler.batcher.push_sample(quiet(ts));
        lock(&sampler.readings).push(Reading::of_sample(&quiet(ts)));
        let program = ProcessSample {
            pid: 4,
            name: "editor.exe".to_string(),
            cpu_pct: 1.0,
            private_bytes: 0,
            read_bps: 0.0,
            write_bps: 0.0,
            gpu_pct: 0.0,
        };
        lock(&sampler.programs).push(ts, &[program]);
        let back = ts - 3_600_000;

        sampler.watch_clock(back + 1_000, 1_000.0);

        assert!(sampler.batcher.take_final().is_empty());
        assert!(lock(&sampler.readings).iter().next().is_none());
        assert!(lock(&sampler.programs).history("editor.exe").is_empty());
        let gaps = history(dir.path()).gaps(i64::MIN, i64::MAX).unwrap();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].reason, GapReason::ClockChanged);
        let (_, count, _) = samples(dir.path());
        assert_eq!(count, 1);
    }

    #[test]
    fn a_steady_clock_leaves_the_batch_alone() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let ts = wall_ms();
        sampler.clock = ClockWatch::new(ts, 0.0);
        sampler.batcher.push_sample(quiet(ts));

        sampler.watch_clock(ts + 1_000, 1_000.0);

        assert!(!sampler.batcher.take_final().is_empty());
    }

    /// Holds the writer lock of the history file in `dir` until it is dropped.
    struct WriterLock(rusqlite::Connection);

    impl WriterLock {
        fn take(dir: &std::path::Path) -> WriterLock {
            let conn = rusqlite::Connection::open(dir.join("vestigium.db")).unwrap();
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            WriterLock(conn)
        }
    }

    impl Drop for WriterLock {
        fn drop(&mut self) {
            let _ = self.0.execute_batch("ROLLBACK");
        }
    }

    #[test]
    fn a_batch_that_fails_to_write_is_stored_by_the_next_flush() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let ts = wall_ms();
        sampler.batcher.push_sample(quiet(ts));
        sampler.events.push(disk_event(1));
        sampler.bookmark_xml = Some("first".to_string());

        let held = WriterLock::take(dir.path());
        sampler.flush_final();
        drop(held);
        assert!(lock(&sampler.status).last_error.is_some());

        sampler.events.push(disk_event(2));
        sampler.bookmark_xml = Some("second".to_string());
        sampler.flush();

        let store = history(dir.path());
        let ids: Vec<u64> = store
            .events(i64::MIN, i64::MAX)
            .unwrap()
            .iter()
            .map(|e| e.record_id)
            .collect();
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(samples(dir.path()).1, 1);
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("second"));
    }

    /// Makes every insert into `table` fail until the returned guard is dropped.
    struct Refusal(rusqlite::Connection, &'static str);

    impl Refusal {
        fn of(dir: &std::path::Path, table: &'static str) -> Refusal {
            let conn = rusqlite::Connection::open(dir.join("vestigium.db")).unwrap();
            conn.execute_batch(&format!(
                "CREATE TRIGGER refuse_{table} BEFORE INSERT ON {table}
                 BEGIN SELECT RAISE(ABORT, 'refused'); END"
            ))
            .unwrap();
            Refusal(conn, table)
        }
    }

    impl Drop for Refusal {
        fn drop(&mut self) {
            let _ = self
                .0
                .execute_batch(&format!("DROP TRIGGER refuse_{}", self.1));
        }
    }

    #[test]
    fn batches_that_keep_failing_wait_up_to_the_cap_then_lose_their_samples() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let refusal = Refusal::of(dir.path(), "segment");
        let ts = wall_ms();
        for n in 1..=4u64 {
            sampler.batcher.push_sample(quiet(ts + n as i64 * 60_000));
            sampler.events.push(disk_event(n));
            sampler.bookmark_xml = Some(format!("bookmark {n}"));
            sampler.flush_final();
        }

        assert_eq!(sampler.waiting.len(), BATCHES_KEPT);
        assert_eq!(sampler.waiting[0].events.len(), 2);
        drop(refusal);
        sampler.flush();

        let store = history(dir.path());
        assert_eq!(store.events(i64::MIN, i64::MAX).unwrap().len(), 4);
        assert_eq!(samples(dir.path()).1, 3);
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("bookmark 4"));
        assert!(sampler.waiting.is_empty());
    }

    #[test]
    fn a_failed_batch_makes_the_next_interval_due_with_nothing_new() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let _refusal = Refusal::of(dir.path(), "event");
        sampler.events.push(disk_event(1));
        sampler.flush();
        assert!(sampler.events.is_empty());

        assert!(sampler.events_due(sampler.last_write_ms + sampler.flush_ms));
    }

    #[test]
    fn clearing_the_history_drops_the_waiting_batches_and_keeps_the_bookmark() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let refusal = Refusal::of(dir.path(), "event");
        sampler.events.push(disk_event(1));
        sampler.bookmark_xml = Some("held".to_string());
        sampler.flush();
        drop(refusal);

        sampler.forget(wall_ms());
        sampler.flush();

        let store = history(dir.path());
        assert!(store.events(i64::MIN, i64::MAX).unwrap().is_empty());
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("held"));
    }

    #[test]
    fn a_batch_with_an_unstorable_value_keeps_only_samples_and_bookmark() {
        let batch = Batch {
            segments: vec![Segment {
                series: Series::Samples,
                start_ms: 1,
                end_ms: 2,
                data: vec![1],
            }],
            events: vec![disk_event(1)],
            bookmark_xml: Some("mark".to_string()),
            gaps: vec![Gap {
                start_ms: 1,
                end_ms: 2,
                reason: GapReason::Paused,
            }],
        };

        let kept = without_unstorable(batch).unwrap();
        assert_eq!(kept.segments.len(), 1);
        assert_eq!(kept.bookmark_xml.as_deref(), Some("mark"));
        assert!(kept.events.is_empty() && kept.gaps.is_empty());
        assert_eq!(without_unstorable(kept), None);
    }

    /// The occurrences the sampler's black box closes for one long freeze, and the poll time.
    fn freeze_outputs(sampler: &mut Sampler, count: usize) -> (Vec<Output>, i64) {
        let base = wall_ms() - 3_600_000;
        let mut outputs = Vec::new();
        for n in 0..count as i64 {
            let at = base + n * 600_000;
            for s in -60..=0 {
                sampler.blackbox.push_sample(quiet(at + s * 1_000));
            }
            sampler.blackbox.push_probe(ProbeTick {
                wall_ms: at + 6_000,
                lag_ms: 6_000.0,
            });
            outputs.extend(sampler.blackbox.poll(at + 300_000));
        }
        (outputs, base + count as i64 * 600_000)
    }

    #[test]
    fn an_occurrence_that_fails_to_store_is_tried_again_and_counted_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let (outputs, now) = freeze_outputs(&mut sampler, 1);
        assert_eq!(outputs.len(), 1);

        let refusal = Refusal::of(dir.path(), "occurrence");
        sampler.keep(outputs, now);
        drop(refusal);
        assert!(
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .is_empty()
        );

        sampler.keep(Vec::new(), now);
        sampler.keep(Vec::new(), now);

        let stored = history(dir.path()).occurrences(i64::MIN, i64::MAX).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(sampler.blackbox.recent(now).occurrences, 1);
    }

    #[test]
    fn occurrences_that_never_store_stop_counting_past_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let (outputs, now) = freeze_outputs(&mut sampler, OUTPUTS_KEPT + 2);
        assert_eq!(outputs.len(), OUTPUTS_KEPT + 2);

        let _refusal = Refusal::of(dir.path(), "occurrence");
        sampler.keep(outputs, now);

        assert_eq!(sampler.unstored.len(), OUTPUTS_KEPT);
        assert_eq!(
            sampler.blackbox.recent(now).occurrences as usize,
            OUTPUTS_KEPT
        );
    }

    #[test]
    fn a_timed_pause_ends_by_itself() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), Duration::from_secs(1));
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        {
            let mut status = lock(&running.status);
            status.paused = true;
            status.paused_until_ms = Some(wall_ms() + 500);
        }
        running.gate.set(GateState::Paused);
        let resumed = wait_until(PATIENCE, || {
            running.gate.state() == GateState::Running && !lock(&running.status).paused
        });
        stop(&running);

        assert!(resumed, "the pause did not end by itself");
        assert_eq!(lock(&running.status).paused_until_ms, None);
    }

    #[test]
    fn pausing_records_a_gap() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), Duration::from_secs(1));
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        running.gate.set(GateState::Paused);
        thread::sleep(Duration::from_millis(1_500));
        running.gate.set(GateState::Running);
        let recorded = wait_until(PATIENCE, || {
            history(dir.path())
                .gaps(i64::MIN, i64::MAX)
                .unwrap()
                .iter()
                .any(|g| g.reason == GapReason::Paused && g.end_ms > g.start_ms)
        });
        stop(&running);

        assert!(recorded, "no gap for the pause");
    }

    fn tick(wall_ms: i64, lag_ms: f32) -> ProbeTick {
        ProbeTick { wall_ms, lag_ms }
    }

    #[test]
    fn hiccups_keep_the_late_wakes_from_8_ms() {
        let mut kept = VecDeque::new();

        keep_hiccups(
            &mut kept,
            &[tick(1_000, 7.9), tick(2_000, 8.0), tick(3_000, 600.0)],
            3_000,
        );

        let times: Vec<i64> = kept.iter().map(|h| h.ts_ms).collect();
        assert_eq!(times, [2_000, 3_000]);
    }

    #[test]
    fn hiccups_drop_what_is_older_than_the_longest_capture() {
        let mut kept = VecDeque::new();
        keep_hiccups(&mut kept, &[tick(0, 20.0), tick(1_000, 20.0)], 1_000);

        keep_hiccups(&mut kept, &[], HICCUPS_KEPT_MS + 500);

        let times: Vec<i64> = kept.iter().map(|h| h.ts_ms).collect();
        assert_eq!(times, [1_000]);
    }

    #[test]
    fn hiccups_stay_for_a_capture_reopened_an_hour_later() {
        let mut kept = VecDeque::new();
        keep_hiccups(&mut kept, &[tick(0, 20.0), tick(1_000, 20.0)], 1_000);

        keep_hiccups(&mut kept, &[], 60 * 60_000);

        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn hiccups_stop_at_the_cap() {
        let mut kept = VecDeque::new();
        let ticks: Vec<ProbeTick> = (0..HICCUPS_KEPT as i64 + 5)
            .map(|k| tick(k, 20.0))
            .collect();

        keep_hiccups(&mut kept, &ticks, 10_000);

        assert_eq!(kept.len(), HICCUPS_KEPT);
        assert_eq!(kept.front().map(|h| h.ts_ms), Some(5));
    }

    #[test]
    fn prune_runs_when_the_hour_changes() {
        let hour = 5;
        let start = hour * HOUR_MS;

        assert!(prune_due(None, start));
        assert!(!prune_due(Some(hour), start));
        assert!(!prune_due(Some(hour), start + HOUR_MS - 1));
        assert!(prune_due(Some(hour), start + HOUR_MS));
    }

    /// One thread of the lock test: counts its turns and reports when it ends.
    fn spin(
        name: &'static str,
        turns: &Arc<std::sync::atomic::AtomicU64>,
        stop: &Arc<AtomicBool>,
        ended: &mpsc::Sender<&'static str>,
        mut turn: impl FnMut(u64) + Send + 'static,
    ) {
        let (turns, stop, ended) = (turns.clone(), stop.clone(), ended.clone());
        thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                turn(turns.fetch_add(1, Ordering::AcqRel));
                thread::yield_now();
            }
            let _ = ended.send(name);
        });
    }

    #[test]
    fn pipe_tray_and_sampler_share_locks_without_a_deadlock() {
        use std::sync::atomic::{AtomicI64, AtomicU64};
        use vestigium_ipc::Request;

        let dir = tempfile::tempdir().unwrap();
        let shared = Arc::new(crate::app::tests::shared());
        let (probe_tx, probe_rx) = mpsc::channel();
        let (_power_tx, power_rx) = mpsc::channel();
        let (_events_tx, events_rx) = mpsc::channel();
        let wiring = Wiring {
            flush: Duration::from_secs(1),
            timing: QUICK,
            gate: shared.gate.clone(),
            probe_rx,
            power_rx,
            events_rx,
            subscription: None,
            sensitive: shared.sensitive.clone(),
            status: shared.status.clone(),
            recent: shared.recent.clone(),
            now: shared.now.clone(),
            readings: shared.readings.clone(),
            programs: shared.programs.clone(),
            pressure: shared.pressure.clone(),
            hiccups: shared.hiccups.clone(),
            settings: shared.settings.clone(),
            cleared: shared.cleared.clone(),
            health_due: shared.health_due.clone(),
            ring_path: Some(dir.path().join("blackbox.ring")),
            source_set: Box::new(|| Box::new(Quiet)),
        };
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let sampler = run_sampler(store, wiring).unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let (ended, ends) = mpsc::channel();
        let pipe_turns = Arc::new(AtomicU64::new(0));
        let tray_turns = Arc::new(AtomicU64::new(0));
        let watch_turns = Arc::new(AtomicU64::new(0));

        let mut pipe_store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let pipe_shared = shared.clone();
        spin("pipe", &pipe_turns, &stop, &ended, move |turn| {
            let request = match turn % 100 {
                0 => Request::SetPaused {
                    paused: true,
                    until_ms: None,
                },
                1 => Request::SetPaused {
                    paused: false,
                    until_ms: None,
                },
                2 => Request::SetSettings(Settings::default()),
                3 => Request::ClearHistory,
                4 => Request::Settings,
                5 => Request::SetSensitive { on: true },
                6 => Request::SetSensitive { on: false },
                7..=9 => Request::Now,
                10..=11 => Request::Recent,
                12 => Request::ProgramRecent {
                    name: "editor.exe".to_string(),
                },
                _ => Request::Status,
            };
            crate::app::handle(request, &pipe_shared, Some(&mut pipe_store));
        });

        let watch_shared = shared.clone();
        let sampled = Arc::new(AtomicI64::new(0));
        let watched = sampled.clone();
        spin("sampler watch", &watch_turns, &stop, &ended, move |_| {
            let at = lock(&watch_shared.status).last_sample_ms.unwrap_or(0);
            watched.store(at, Ordering::Release);
        });

        let tray_shared = shared.clone();
        spin("tray", &tray_turns, &stop, &ended, move |_| {
            crate::tray::read_shared(&tray_shared);
        });

        let count = |turns: &Arc<AtomicU64>| turns.load(Ordering::Acquire);
        let late_tick = || {
            let _ = probe_tx.send(ProbeTick {
                wall_ms: wall_ms(),
                lag_ms: 1_500.0,
            });
        };
        late_tick();
        thread::sleep(Duration::from_millis(1_200));
        let before = (count(&pipe_turns), count(&tray_turns));
        let sampled_before = sampled.load(Ordering::Acquire);
        late_tick();
        thread::sleep(Duration::from_millis(1_200));
        late_tick();
        thread::sleep(Duration::from_millis(1_100));
        let after = (count(&pipe_turns), count(&tray_turns));
        let sampled_after = sampled.load(Ordering::Acquire);

        stop.store(true, Ordering::Release);
        shared.gate.set(GateState::Stopping);
        let mut ended_names = Vec::new();
        while ended_names.len() < 3 {
            match ends.recv_timeout(Duration::from_secs(5)) {
                Ok(name) => ended_names.push(name),
                Err(_) => break,
            }
        }

        for name in ["pipe", "tray", "sampler watch"] {
            assert!(
                ended_names.contains(&name),
                "the {name} loop never came back: two locks were taken in opposite orders"
            );
        }
        assert!(
            sampler.wait(Duration::from_secs(5)),
            "the sampler never stopped"
        );
        assert!(after.0 > before.0, "the pipe stopped answering");
        assert!(after.1 > before.1, "the tray stopped reading");
        assert!(
            sampled_before > 0 && sampled_after > sampled_before,
            "the sampler stopped taking samples"
        );
    }
}
