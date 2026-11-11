use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io;
use std::mem;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::{
    BlackboxRing, CollectError, Delivered, EventSubscription, HealthSource, PdhReading, Waited,
    Wake, disk_media, hung_pids, interrupt_ms, logical_cpus, memory_info, physical_drive,
    unbiased_ms, wall_ms,
};
use vestigium_core::{
    Blackbox, CAPTURE_MINUTES_MAX, CpuWindow, DiskInfo, EventMeaning, Gap, GapReason, GateState,
    HICCUP_MS, HungWindow, INTERVAL_KEPT, INTERVAL_MS, MachineInfo, Media, Notable, Output,
    PressureSpan, ProbeHiccup, ProbeTick, ProcessCounters, ProcessTop, ProgramRing,
    REPORTED_EXEMPLARS, RING_MS, Reading, RecentState, Ring, RunGate, Sample, Settings, Severity,
    Timing, WinEvent, heaviest, last_seen, process_rates, programs,
};
use vestigium_ipc::{AgentStatus, NowFrame};
use vestigium_store::{Batch, Batcher, Segment, Store, StoreError};
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

use vestigium_core::health::Concern;

use crate::app::lock;
use crate::clock::ClockWatch;
use crate::fall::{Fall, RingSync, Strain};
use crate::health::{AnySource, HealthReader, HealthThread, PassRequest};
use crate::power::PowerNotice;
use crate::readings::{Backoff, Readings, Source, SourceHealths, SourceSet, Waiter};
use crate::sensitive::SensitiveMode;
use crate::service::{BATCHES_KEPT, Epoch, Job, Links, Queued, Service, shed_into};
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
/// How long a pause, a suspend or the end of the thread waits for the storage thread to write
/// what was handed to it.
const DRAIN_WAIT: Duration = Duration::from_secs(1);
/// How long a failed Event Log subscription waits before the next attempt.
const SUBSCRIBE_RETRY_MS: i64 = 60_000;
/// Events of one provider and id stored per batch: above what the black box keeps for one report
/// and far above the counts the health checks need.
const BATCH_EVENT_EXEMPLARS: usize = 4 * REPORTED_EXEMPLARS;

/// Whether the batch holds fewer than the cap of events with the provider and id of `event`.
fn batch_has_room(batch: &[WinEvent], event: &WinEvent) -> bool {
    batch
        .iter()
        .filter(|kept| kept.provider == event.provider && kept.id == event.id)
        .count()
        < BATCH_EVENT_EXEMPLARS
}

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

/// Starts the Event Log subscription again after the stored bookmark, or from the backfill.
pub type Subscriber =
    Box<dyn FnMut(Option<&str>) -> Result<EventSubscription, CollectError> + Send>;

/// What the sampler is connected to.
pub struct Wiring {
    /// How often a batch is written.
    pub flush: Duration,
    pub timing: Timing,
    pub gate: Arc<RunGate>,
    pub probe_rx: Receiver<ProbeTick>,
    pub power_rx: Receiver<PowerNotice>,
    /// Set to end the sampler's wait for the next tick at once.
    pub wake: Arc<Wake>,
    pub events_rx: Receiver<Delivered>,
    /// Kept alive as long as the sampler; `None` when the Event Log could not be read.
    pub subscription: Option<EventSubscription>,
    /// Tried about once a minute while `subscription` is `None`; `None` never tries again.
    pub resubscribe: Option<Subscriber>,
    pub sensitive: Arc<SensitiveMode>,
    /// `paused_until_ms` is an instant of the steady clock here, as in the shared status.
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
    /// Counts the clears of the history; the pipe advances it, the sampler forgets what it held.
    pub epoch: Arc<Epoch>,
    /// Set by the pipe after it stored a health scan.
    pub health_due: Arc<AtomicBool>,
    /// The black box file that outlives a fall; `None` keeps none.
    pub ring_path: Option<PathBuf>,
    /// What each tick reads from Windows, and how the counters and the timer are opened.
    pub source_set: SourceSet,
    /// What the health pass reads, on a thread of its own.
    pub health: Box<dyn HealthSource + Send>,
}

/// Starts the 1 Hz sampler, which feeds the black box and writes into `store`.
pub fn run_sampler(store: Store, wiring: Wiring) -> io::Result<SamplerHandle> {
    let (done_tx, done_rx) = mpsc::channel();
    let storage = start_storage(store, &wiring)?;
    thread::Builder::new()
        .name("sampler".to_string())
        .spawn(move || {
            let gate = wiring.gate.clone();
            let mut sampler = Sampler::new(storage, wiring);
            sampler.run(&gate);
            sampler.take_events();
            sampler.settle_fall(wall_ms(), true);
            sampler.leave(GapReason::Stopped);
            sampler.close_ring();
            sampler.service.stop(DRAIN_WAIT);
            let _ = done_tx.send(());
        })?;
    Ok(SamplerHandle {
        done: Mutex::new(done_rx),
    })
}

/// The storage thread, and what the sampler needs to read and hear back from it.
struct Storage {
    service: Service,
    /// Occurrences and episodes the thread gave up after too many failed writes.
    dropped: Receiver<Output>,
    /// The history file, which the sampler reads through connections of its own.
    db: Option<PathBuf>,
    /// Critical health findings open now, kept by the storage thread.
    critical: Arc<AtomicU32>,
}

/// Hands the history to a storage thread.
fn start_storage(store: Store, wiring: &Wiring) -> io::Result<Storage> {
    let db = store.path();
    let (dropped_tx, dropped) = mpsc::channel();
    let critical = Arc::new(AtomicU32::new(0));
    let service = Service::start(
        store,
        Links {
            status: wiring.status.clone(),
            epoch: wiring.epoch.clone(),
            health_due: wiring.health_due.clone(),
            now: wiring.now.clone(),
            critical: critical.clone(),
            dropped: dropped_tx,
        },
    )?;
    Ok(Storage {
        service,
        dropped,
        db,
        critical,
    })
}

/// True when the wall clock entered an hour that has not been pruned yet.
pub fn prune_due(pruned_hour: Option<i64>, now_ms: i64) -> bool {
    pruned_hour != Some(now_ms.div_euclid(HOUR_MS))
}

struct Sampler {
    /// The storage thread, which owns the write connection.
    service: Service,
    dropped: Receiver<Output>,
    /// The history file, read through a connection opened for each read.
    db: Option<PathBuf>,
    /// Jobs the full queue did not take, oldest first, tried again every tick.
    backlog: VecDeque<Queued>,
    epoch: Arc<Epoch>,
    /// The epoch of the history this sampler holds in memory.
    seen_epoch: u64,
    batcher: Batcher,
    blackbox: Blackbox,
    probe_rx: Receiver<ProbeTick>,
    power_rx: Receiver<PowerNotice>,
    wake: Arc<Wake>,
    events_rx: Receiver<Delivered>,
    subscription: Option<EventSubscription>,
    resubscribe: Option<Subscriber>,
    next_subscribe_ms: i64,
    sensitive: Arc<SensitiveMode>,
    status: Arc<Mutex<AgentStatus>>,
    recent: Arc<Mutex<RecentState>>,
    now: Arc<Mutex<NowFrame>>,
    readings: Arc<Mutex<Ring<Reading>>>,
    programs: Arc<Mutex<ProgramRing>>,
    pressure: Arc<Mutex<Vec<PressureSpan>>>,
    hiccups: Arc<Mutex<VecDeque<ProbeHiccup>>>,
    settings: Arc<Mutex<Settings>>,
    health_due: Arc<AtomicBool>,
    /// Events and the bookmark that covers them, waiting for the next batch.
    events: Vec<WinEvent>,
    bookmark_xml: Option<String>,
    gaps: Vec<Gap>,
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
    /// Reads the health sources; `None` when its thread could not start.
    health: Option<HealthThread>,
    /// When the first health pass is due; afterwards a pass runs with every hourly prune.
    first_health_ms: Option<i64>,
    /// When each disk last had I/O.
    disk_active_ms: HashMap<String, i64>,
    commit_limit_bytes: u64,
    /// Critical health findings open now, for the tray; the storage thread keeps it.
    critical_findings: Arc<AtomicU32>,
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
    fn new(storage: Storage, wiring: Wiring) -> Sampler {
        let Storage {
            service,
            dropped,
            db,
            critical,
        } = storage;
        let health = HealthThread::start(
            HealthReader::new(AnySource(wiring.health)),
            service.clone(),
            wiring.status.clone(),
        );
        let history = db.as_deref().map(Store::open_read_only);
        let now = wall_ms();
        let opened = wiring.ring_path.as_deref().map(BlackboxRing::open);
        let machine = history
            .as_ref()
            .and_then(|history| history.as_ref().ok())
            .and_then(|history| history.machine().ok().flatten())
            .unwrap_or(MachineInfo {
                logical_cpus: logical_cpus(),
                mem_total_bytes: memory_info().map_or(0, |m| m.total_bytes),
                disks: Vec::new(),
            });
        let mut sampler = Sampler {
            service,
            dropped,
            db,
            backlog: VecDeque::new(),
            seen_epoch: wiring.epoch.current(),
            epoch: wiring.epoch,
            batcher: Batcher::new(wiring.flush),
            blackbox: Blackbox::new(machine, wiring.timing),
            probe_rx: wiring.probe_rx,
            power_rx: wiring.power_rx,
            wake: wiring.wake,
            events_rx: wiring.events_rx,
            next_subscribe_ms: now + SUBSCRIBE_RETRY_MS,
            subscription: wiring.subscription,
            resubscribe: wiring.resubscribe,
            sensitive: wiring.sensitive,
            status: wiring.status,
            recent: wiring.recent,
            now: wiring.now,
            readings: wiring.readings,
            programs: wiring.programs,
            pressure: wiring.pressure,
            hiccups: wiring.hiccups,
            settings: wiring.settings,
            health_due: wiring.health_due,
            events: Vec::new(),
            bookmark_xml: None,
            gaps: Vec::new(),
            flush_ms: i64::try_from(wiring.flush.as_millis()).unwrap_or(i64::MAX),
            last_write_ms: now,
            away_since: None,
            pruned_hour: None,
            snapshots: VecDeque::with_capacity(TOP_EVERY + 1),
            gpu_by_pid: HashMap::new(),
            logical_cpus: logical_cpus(),
            own_cpu: CpuWindow::new(OWN_CPU_SPAN_MS),
            health: None,
            first_health_ms: Some(now + FIRST_HEALTH_AFTER_MS),
            disk_active_ms: HashMap::new(),
            commit_limit_bytes: 0,
            critical_findings: critical,
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
        };
        match health {
            Ok(thread) => sampler.health = Some(thread),
            Err(err) => sampler.report(&err),
        }
        let history = history.and_then(Result::ok);
        if let Some(history) = &history {
            sampler.recall_last_day(history, now);
        }
        let history_last = history.and_then(|history| history.last_sample_end_ms().ok().flatten());
        match opened {
            Some(Ok((ring, held))) => {
                if let Some(warning) = &held.warning {
                    sampler.report(warning);
                }
                sampler.fall = Fall::after(held, history_last, now, interrupt_ms());
                if sampler.fall.is_none() {
                    ring.release_held();
                }
                sampler.ring = Some(ring);
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
    fn recall_last_day(&mut self, history: &Store, now: i64) {
        if let Ok(open) = history.open_findings() {
            let critical = open
                .iter()
                .filter(|f| f.finding.concern == Concern::Critical)
                .count() as u32;
            self.critical_findings.store(critical, Ordering::Release);
        }
        if let Ok(occurrences) = history.occurrences(now - DAY_MS, now) {
            lock(&self.now).latest = occurrences.last().cloned();
            for o in occurrences {
                let severe = o.severity == Severity::Severe;
                self.blackbox
                    .remember(o.end_ms, Notable::Occurrence { severe });
            }
        }
        if let Ok(episodes) = history.episodes(now - DAY_MS, now) {
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
            let woken = self.pace(&mut sources, Duration::from_secs_f64(due_ms / 1000.0), gate);
            if woken {
                // Woken before the tick: handle what woke us and wait out the rest of the second.
                if gate.state() == GateState::Running && self.on_power(&mut sources) {
                    next_tick = unbiased_ms() + TICK_MS;
                    trim_at = Some(ticks + TRIM_AFTER_TICKS);
                }
                continue;
            }
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
            if self.sensitive.expire(interrupt_ms()) {
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

    /// Waits `due` for the next tick on the timer, or with a plain sleep while there is none;
    /// true when the wake event ended the wait early. The timer is created again on a growing
    /// wait; a pause or a stop ends the sleep at once.
    fn pace(&mut self, sources: &mut Sources, due: Duration, gate: &RunGate) -> bool {
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
            let waited = timer.wait(due, &self.wake);
            if let Ok(waited) = self.read(Source::Timer, waited) {
                return waited == Waited::Woken;
            }
            sources.timer = None;
            sources.timer_backoff.failed();
        }
        gate.wait_while_running_for(due);
        false
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
        self.forget_if_cleared();
        self.push_backlog();
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
        // Memory and the processor feed every rule, so a second without either has no sample; a
        // second without the counters has one whose counter fields are absent.
        if let (Some(memory), Some(cpu)) = (memory, cpu) {
            let pdh = pdh.unwrap_or(PdhReading {
                perf_limit_pct: None,
                proc_perf_pct: None,
                hard_faults_per_s: None,
                disks: Vec::new(),
                gpu: None,
            });
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
                .filter(|d| d.active_pct > 0.0 || d.bytes_per_s.is_some_and(|b| b > 0.0))
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
        self.keep_subscribed(ts_ms);
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
        if self.prune_if_due(ts_ms) && self.first_health_ms.is_none() {
            self.check_health(ts_ms);
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

    /// Asks the storage thread to prune when the wall clock entered a new hour; true when it did.
    /// A refused prune is not an error, and the next wall hour tries again.
    fn prune_if_due(&mut self, ts_ms: i64) -> bool {
        if !prune_due(self.pruned_hour, ts_ms) {
            return false;
        }
        self.pruned_hour = Some(ts_ms.div_euclid(HOUR_MS));
        let retention = lock(&self.settings).retention;
        self.send(Job::Prune {
            now_ms: ts_ms,
            retention,
        });
        true
    }

    /// Asks the storage thread to list the dumps Windows kept, without opening any.
    fn check_dumps(&mut self, ts_ms: i64) {
        self.send(Job::Dumps {
            places: None,
            now_ms: ts_ms,
        });
    }

    /// Lists the dumps and asks for a health pass: a minute after start, then every full hour. The
    /// pass reads on its own thread, so a source that blocks holds up nothing here.
    fn check_health(&mut self, ts_ms: i64) {
        self.check_dumps(ts_ms);
        let busy_disks = self
            .disk_active_ms
            .iter()
            .filter(|(_, at)| ts_ms - **at <= DISK_BUSY_MS)
            .map(|(name, _)| name.clone())
            .collect();
        let request = PassRequest {
            ts_ms,
            machine: self.blackbox.machine().clone(),
            busy_disks,
            commit_limit_bytes: self.commit_limit_bytes,
        };
        if let Some(health) = &self.health {
            health.request(self.seen_epoch, request);
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

    /// Reports an event the subscription could not read, and starts a lost subscription again
    /// once a minute from the stored bookmark.
    fn keep_subscribed(&mut self, now: i64) {
        if let Some(subscription) = &self.subscription {
            if let Some(reason) = subscription.take_failure() {
                self.report(&reason);
            }
            return;
        }
        if now < self.next_subscribe_ms {
            return;
        }
        if self.resubscribe.is_none() {
            return;
        }
        self.next_subscribe_ms = now + SUBSCRIBE_RETRY_MS;
        let bookmark = self
            .history()
            .and_then(|history| history.bookmark_xml())
            .ok()
            .flatten();
        let Some(start) = self.resubscribe.as_mut() else {
            return;
        };
        match start(bookmark.as_deref()) {
            Ok(subscription) => self.subscription = Some(subscription),
            Err(err) => self.report(&format!("the Event Log subscription failed: {err}")),
        }
    }

    /// Moves delivered events into the black box and the next batch, with their bookmark.
    fn take_events(&mut self) {
        let steady_ms = interrupt_ms();
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
                fall.offer(event, steady_ms);
            }
            self.blackbox.push_event(delivered.event.clone());
            if batch_has_room(&self.events, &delivered.event) {
                self.events.push(delivered.event);
            }
            if delivered.bookmark_xml.is_some() {
                self.bookmark_xml = delivered.bookmark_xml;
            }
        }
    }

    /// Hands occurrences and episodes to the storage thread right away; gaps wait for the batch.
    /// While some wait on the thread for another try, they are asked for again each second.
    fn keep(&mut self, outputs: Vec<Output>, now: i64) {
        for output in outputs {
            match output {
                Output::Occurrence(occurrence) => self.send(Job::Occurrence(occurrence)),
                Output::Episode(episode) => self.send(Job::Episode(episode)),
                Output::Gap(gap) => self.gaps.push(gap),
            }
        }
        if self.backlog.is_empty() && self.service.holds_outputs() {
            self.send(Job::Retry);
        }
        for given_up in self.dropped.try_iter() {
            self.blackbox.forget_notable(&given_up);
        }
        self.publish_recent(now);
    }

    /// Turns a fall Windows has spoken about into a post-mortem, and records the time away;
    /// `force` settles it with what is known so far.
    fn settle_fall(&mut self, now: i64, force: bool) {
        if !force
            && !self
                .fall
                .as_ref()
                .is_some_and(|f| f.settled(interrupt_ms()))
        {
            return;
        }
        let Some(fall) = self.fall.take() else {
            return;
        };
        if let Some(ring) = &self.ring {
            ring.release_held();
        }
        let reason = match self.history().and_then(|history| fall.occurrence(&history)) {
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
                .backlog
                .iter()
                .rev()
                .find_map(|queued| match &queued.job {
                    Job::Batch(batch) => batch.bookmark_xml.clone(),
                    _ => None,
                });
        }
        self.backlog.clear();
        self.blackbox = self.blackbox.emptied();
        if let Some(ring) = self.ring.as_mut() {
            ring.clear();
            ring.release_held();
        }
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
            critical_findings: self.critical_findings.load(Ordering::Acquire),
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
        self.send(Job::Machine(machine.clone()));
        self.blackbox.set_machine(machine);
    }

    /// Handles pending suspend and resume notices; true when a resume restarted the sources. A
    /// suspend is answered once everything is written.
    fn on_power(&mut self, sources: &mut Sources) -> bool {
        let mut resumed = false;
        while let Ok(event) = self.power_rx.try_recv() {
            match event {
                PowerNotice::Suspend(done) => {
                    self.leave(GapReason::Suspended);
                    let _ = done.send(());
                }
                PowerNotice::Resume => {
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
    /// Blocks while paused and ends a timed pause when its time comes; the steady clock, which
    /// counts sleep and ignores wall-clock changes, is read again at least every minute.
    fn wait_out_pause(&self, gate: &RunGate) -> GateState {
        loop {
            let until = lock(&self.status).paused_until_ms;
            let left_ms = until.map_or(PAUSE_CHECK_MS, |end| {
                (end - interrupt_ms()).clamp(0, PAUSE_CHECK_MS)
            });
            if until.is_some() && left_ms == 0 {
                // A pause asked for meanwhile replaces this one; the status and the gate change
                // under one guard, so a new pause is never ended by the old one.
                let mut status = lock(&self.status);
                if status
                    .paused_until_ms
                    .is_some_and(|end| end <= interrupt_ms())
                {
                    status.paused = false;
                    status.paused_until_ms = None;
                    gate.set(GateState::Running);
                    drop(status);
                    shutdown::wake();
                    return GateState::Running;
                }
                continue;
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
        self.drain();
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
        (!self.events.is_empty() || !self.gaps.is_empty() || self.service.is_retrying())
            && now_ms.saturating_sub(self.last_write_ms) >= self.flush_ms
    }

    /// Drops what was buffered before a clear that finished since the last look.
    fn forget_if_cleared(&mut self) {
        let epoch = self.epoch.current();
        if epoch != self.seen_epoch {
            self.seen_epoch = epoch;
            self.forget(wall_ms());
        }
    }

    /// Writes whatever is buffered except the minute still open.
    fn flush(&mut self) {
        self.forget_if_cleared();
        let segments = self.batcher.take();
        self.write(segments);
    }

    /// Writes everything buffered, the open minute included.
    fn flush_final(&mut self) {
        self.forget_if_cleared();
        let segments = self.batcher.take_final();
        self.write(segments);
    }

    /// Hands segments, events with their bookmark, and gaps to the storage thread as one batch; a
    /// batch with nothing new still goes while earlier ones wait for another try.
    fn write(&mut self, segments: Vec<Segment>) {
        self.last_write_ms = wall_ms();
        let batch = Batch {
            segments,
            events: mem::take(&mut self.events),
            bookmark_xml: self.bookmark_xml.take(),
            gaps: mem::take(&mut self.gaps),
        };
        if !batch.is_empty() {
            self.send(Job::Batch(batch));
        } else if self.service.is_retrying() {
            self.send(Job::Retry);
        } else {
            self.push_backlog();
        }
    }

    /// Queues a job behind the ones that did not fit before it.
    fn send(&mut self, job: Job) {
        self.backlog.push_back(Queued {
            epoch: self.seen_epoch,
            job,
        });
        self.push_backlog();
    }

    /// Hands the backlog to the storage thread in order, until its queue is full; beyond the cap
    /// the oldest batch loses its samples.
    fn push_backlog(&mut self) {
        while let Some(queued) = self.backlog.pop_front() {
            if let Err(back) = self.service.submit(queued) {
                self.backlog.push_front(back);
                break;
            }
        }
        while self.backlog_batches() > BATCHES_KEPT {
            self.shed_oldest();
        }
    }

    fn backlog_batches(&self) -> usize {
        self.backlog
            .iter()
            .filter(|queued| matches!(queued.job, Job::Batch(_)))
            .count()
    }

    /// Drops the samples of the oldest batch in the backlog and records a gap over their span; its
    /// events, gaps and bookmark move into the next one.
    fn shed_oldest(&mut self) {
        let is_batch = |queued: &Queued| matches!(queued.job, Job::Batch(_));
        let Some(first) = self.backlog.iter().position(is_batch) else {
            return;
        };
        let Some(Queued {
            job: Job::Batch(oldest),
            ..
        }) = self.backlog.remove(first)
        else {
            return;
        };
        if let Some(Queued {
            job: Job::Batch(next),
            ..
        }) = self.backlog.iter_mut().find(|queued| is_batch(queued))
        {
            shed_into(oldest, next);
        }
    }

    /// A connection that only reads, opened for the read and closed after it.
    fn history(&self) -> Result<Store, StoreError> {
        match &self.db {
            Some(path) => Store::open_read_only(path),
            None => Err(StoreError::NotFound),
        }
    }

    /// Waits for the storage thread to write what was handed to it, for at most `DRAIN_WAIT`.
    fn drain(&mut self) {
        let deadline = Instant::now() + DRAIN_WAIT;
        while !self.backlog.is_empty() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
            self.push_backlog();
        }
        self.service
            .flush(deadline.saturating_duration_since(Instant::now()));
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

/// Adds the late wakes among `ticks` to `kept`, then drops what is old and keeps the worst
/// `INTERVAL_KEPT` of each interval.
fn keep_hiccups(kept: &mut VecDeque<ProbeHiccup>, ticks: &[ProbeTick], now_ms: i64) {
    let added: Vec<ProbeHiccup> = ticks
        .iter()
        .filter(|tick| tick.lag_ms >= HICCUP_MS.start)
        .map(|tick| ProbeHiccup {
            ts_ms: tick.wall_ms,
            lag_ms: tick.lag_ms,
        })
        .collect();
    if let Some(first) = added.iter().map(|h| h.ts_ms.div_euclid(INTERVAL_MS)).min() {
        let from = kept
            .iter()
            .rposition(|h| h.ts_ms.div_euclid(INTERVAL_MS) < first)
            .map_or(0, |i| i + 1);
        let mut touched: BTreeMap<i64, Vec<ProbeHiccup>> = BTreeMap::new();
        for h in kept.drain(from..).chain(added) {
            touched
                .entry(h.ts_ms.div_euclid(INTERVAL_MS))
                .or_default()
                .push(h);
        }
        for mut own in touched.into_values() {
            if own.len() > INTERVAL_KEPT {
                own.sort_by(|a, b| b.lag_ms.total_cmp(&a.lag_ms).then(a.ts_ms.cmp(&b.ts_ms)));
                own.truncate(INTERVAL_KEPT);
                own.sort_by_key(|h| h.ts_ms);
            }
            kept.extend(own);
        }
    }
    while kept
        .front()
        .is_some_and(|h| now_ms - h.ts_ms > HICCUPS_KEPT_MS)
    {
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
pub(crate) fn release_idle_pages() {
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
    use crate::service::OUTPUTS_KEPT;
    use crate::testing::{Refusal, WriterLock, disk_event, quiet, wait_until};
    use vestigium_collect::{CpuReading, HighResTimer, MemoryInfo};
    use vestigium_core::DiskLoad;

    struct Running {
        gate: Arc<RunGate>,
        handle: SamplerHandle,
        status: Arc<Mutex<AgentStatus>>,
        now: Arc<Mutex<NowFrame>>,
        probe_tx: mpsc::SyncSender<ProbeTick>,
        power_tx: mpsc::Sender<PowerNotice>,
        wake: Arc<Wake>,
        events_tx: mpsc::SyncSender<Delivered>,
    }

    /// What a test keeps of a sampler's wiring: the gate, the shared state and the senders.
    struct Ties {
        epoch: Arc<Epoch>,
        gate: Arc<RunGate>,
        status: Arc<Mutex<AgentStatus>>,
        now: Arc<Mutex<NowFrame>>,
        probe_tx: mpsc::SyncSender<ProbeTick>,
        power_tx: mpsc::Sender<PowerNotice>,
        wake: Arc<Wake>,
        events_tx: mpsc::SyncSender<Delivered>,
    }

    /// Occurrences close one second after their trigger, so tests do not wait half a minute.
    const QUICK: Timing = Timing {
        before_ms: 5_000,
        after_ms: 1_000,
        merge_ms: 1_000,
        max_ms: 60_000,
    };

    /// A machine whose health sources read nothing, so no test depends on this PC's disks.
    struct NoHealth;

    impl HealthSource for NoHealth {
        fn disk(&mut self, _: u32, _: &str, _: i64) -> Option<vestigium_core::health::DiskHealth> {
            None
        }

        fn bus(&mut self, _: u32) -> vestigium_core::hwtest::Bus {
            vestigium_core::hwtest::Bus::Other
        }

        fn volumes(&mut self) -> Vec<vestigium_core::health::VolumeSpace> {
            Vec::new()
        }

        fn system_drive(&mut self) -> Option<u32> {
            None
        }

        fn page_files(&mut self) -> Option<Vec<vestigium_core::health::PageFile>> {
            None
        }

        fn antivirus(&mut self) -> Option<Vec<vestigium_core::health::SecurityProduct>> {
            None
        }
    }

    /// A disk that answers after seconds, as a failing one can.
    struct Sleepy {
        asked: Arc<AtomicBool>,
        release: Arc<AtomicBool>,
    }

    impl HealthSource for Sleepy {
        fn disk(
            &mut self,
            _: u32,
            name: &str,
            now_ms: i64,
        ) -> Option<vestigium_core::health::DiskHealth> {
            self.asked.store(true, Ordering::Release);
            let until = Instant::now() + Duration::from_secs(5);
            while Instant::now() < until && !self.release.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
            }
            Some(vestigium_core::health::DiskHealth {
                name: name.to_string(),
                read_ms: now_ms,
                predict_failure: false,
                attributes: Vec::new(),
                nvme: None,
            })
        }

        fn bus(&mut self, _: u32) -> vestigium_core::hwtest::Bus {
            vestigium_core::hwtest::Bus::Other
        }

        fn volumes(&mut self) -> Vec<vestigium_core::health::VolumeSpace> {
            Vec::new()
        }

        fn system_drive(&mut self) -> Option<u32> {
            None
        }

        fn page_files(&mut self) -> Option<Vec<vestigium_core::health::PageFile>> {
            None
        }

        fn antivirus(&mut self) -> Option<Vec<vestigium_core::health::SecurityProduct>> {
            None
        }
    }

    /// A sampler on the quiet machine, so no test depends on what this PC reads.
    fn start(dir: &std::path::Path, flush: Duration) -> Running {
        start_with(dir, flush, quiet_set())
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
            wake: ties.wake,
            events_tx: ties.events_tx,
        }
    }

    /// The wiring of a sampler and the handles a test uses to drive it.
    fn wire(dir: &std::path::Path, flush: Duration, source_set: SourceSet) -> (Wiring, Ties) {
        let gate = RunGate::new();
        let (probe_tx, probe_rx) = mpsc::sync_channel(64);
        let (power_tx, power_rx) = mpsc::channel();
        let wake = Arc::new(Wake::new().unwrap());
        let (events_tx, events_rx) = mpsc::sync_channel(64);
        let status = Arc::new(Mutex::new(AgentStatus::default()));
        let sensitive = SensitiveMode::new(ProbeConfig::new(DEFAULT_PROBE_PERIOD), SENSITIVE_FOR);
        let now = Arc::new(Mutex::new(NowFrame::default()));
        let epoch = Arc::new(Epoch::default());
        let wiring = Wiring {
            flush,
            timing: QUICK,
            gate: gate.clone(),
            probe_rx,
            power_rx,
            wake: wake.clone(),
            events_rx,
            subscription: None,
            resubscribe: None,
            sensitive,
            status: status.clone(),
            recent: Arc::new(Mutex::new(RecentState::default())),
            now: now.clone(),
            readings: Arc::new(Mutex::new(Ring::with_capacity(660))),
            programs: Arc::new(Mutex::new(ProgramRing::new(
                RING_MS,
                vestigium_core::PROGRAM_FRAMES_MAX,
            ))),
            pressure: Arc::new(Mutex::new(Vec::new())),
            hiccups: Arc::new(Mutex::new(VecDeque::new())),
            settings: Arc::new(Mutex::new(Settings::default())),
            epoch: epoch.clone(),
            health_due: Arc::new(AtomicBool::new(false)),
            ring_path: Some(dir.join("blackbox.ring")),
            source_set,
            health: Box::new(NoHealth),
        };
        let ties = Ties {
            epoch,
            gate,
            status,
            now,
            probe_tx,
            power_tx,
            wake,
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

    const PATIENCE: Duration = Duration::from_secs(15);
    const NEVER_DUE: Duration = Duration::from_secs(3600);

    fn sampled(status: &Mutex<AgentStatus>) -> bool {
        lock(status).last_sample_ms.is_some()
    }

    /// Sleeps for the wait, as the high-resolution timer would.
    struct Sleeper;

    impl Waiter for Sleeper {
        fn wait(&self, due: Duration, _wake: &Wake) -> Result<Waited, CollectError> {
            thread::sleep(due);
            Ok(Waited::Elapsed)
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
                perf_limit_pct: Some(100.0),
                proc_perf_pct: Some(100.0),
                hard_faults_per_s: Some(0.0),
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

    fn quiet_set() -> SourceSet {
        Box::new(|| Box::new(Quiet))
    }

    /// The quiet machine with the real timer, for the test of the wake event.
    struct QuietOnTheTimer(Quiet);

    impl Readings for QuietOnTheTimer {
        fn memory(&mut self) -> Result<MemoryInfo, CollectError> {
            self.0.memory()
        }

        fn cpu(&mut self) -> Result<Option<CpuReading>, CollectError> {
            self.0.cpu()
        }

        fn pdh(&mut self) -> Result<Option<PdhReading>, CollectError> {
            self.0.pdh()
        }

        fn processes(&mut self) -> Result<Vec<ProcessCounters>, CollectError> {
            self.0.processes()
        }

        fn reset(&mut self) {}

        fn open_pdh(&mut self) -> Result<(), CollectError> {
            Ok(())
        }

        fn open_timer(&mut self) -> Result<Box<dyn Waiter>, CollectError> {
            Ok(Box::new(HighResTimer::new()?))
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
                    latency_ms: Some(1.0),
                    queue_len: Some(0.0),
                    bytes_per_s: Some(0.0),
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

        let opened = wait_until(PATIENCE, || {
            lock(&script).pdh_open_attempts >= 3
                && lock(&running.now)
                    .sample
                    .as_ref()
                    .is_some_and(|s| s.perf_limit_pct.is_some())
        });
        stop(&running);

        assert!(opened, "the counters opened but no sample carried them");
        assert_eq!(lock(&script).pdh_open_attempts, 3);
        assert!(lock(&running.status).last_error.is_some());
        let stored = history(dir.path()).samples(i64::MIN, i64::MAX).unwrap();
        assert_eq!(stored.first().map(|s| s.perf_limit_pct), Some(None));
        assert!(stored.last().is_some_and(|s| s.perf_limit_pct.is_some()));
    }

    #[test]
    fn counters_that_never_open_leave_samples_with_those_fields_absent() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script {
            pdh_open_failures: u32::MAX,
            ..Script::default()
        }));
        let running = start_with(dir.path(), NEVER_DUE, scripted(&script));

        let sampled_once = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_once, "no sample while the counters were down");
        let stored = history(dir.path()).samples(i64::MIN, i64::MAX).unwrap();
        assert!(!stored.is_empty());
        for sample in &stored {
            assert_eq!(sample.perf_limit_pct, None);
            assert_eq!(sample.proc_perf_pct, None);
            assert_eq!(sample.hard_faults_per_s, None);
            assert!(sample.disks.is_empty());
            assert_eq!(sample.cpu_pct, 10.0);
        }
    }

    #[test]
    fn a_counter_read_that_fails_leaves_the_sample_without_its_counters() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script {
            failing: vec![Source::Pdh],
            ..Script::default()
        }));
        let running = start_with(dir.path(), NEVER_DUE, scripted(&script));

        let sampled_once = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_once, "no sample while the counter reads failed");
        let stored = history(dir.path()).samples(i64::MIN, i64::MAX).unwrap();
        assert!(stored.iter().all(|s| s.perf_limit_pct.is_none()));
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
                bookmark_xml: Some("<BookmarkList/>".to_string()),
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
        let running = start_with(dir.path(), NEVER_DUE, quiet_set());

        let sampled_once = wait_until(PATIENCE, || sampled(&running.status));
        stop(&running);

        assert!(sampled_once, "no sample from the replaced sources");
        let (_, count, _) = samples(dir.path());
        assert!(count >= 1);
        assert_eq!(lock(&running.status).last_error, None);
    }

    #[test]
    fn a_ring_of_another_version_is_reported_at_start() {
        let dir = tempfile::tempdir().unwrap();
        let mut ring = vec![0u8; 32 * 1024];
        ring[..4].copy_from_slice(b"VBXR");
        ring[4] = 9;
        std::fs::write(dir.path().join("blackbox.ring"), ring).unwrap();

        let running = start(dir.path(), NEVER_DUE);
        let reported = wait_until(PATIENCE, || {
            lock(&running.status)
                .last_error
                .as_deref()
                .is_some_and(|e| e.contains("black box"))
        });
        stop(&running);

        assert!(reported, "the foreign ring was not reported");
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
        let running = start_with(dir.path(), NEVER_DUE, windows_source_set());

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

        let (done, _answered) = mpsc::channel();
        running.power_tx.send(PowerNotice::Suspend(done)).unwrap();
        running.wake.set();
        assert!(
            wait_until(PATIENCE, || samples(dir.path()).0 == 1),
            "suspending did not write the buffered batch"
        );
        let resumed_at = wall_ms();
        running.power_tx.send(PowerNotice::Resume).unwrap();
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
    fn a_suspend_records_its_own_instant() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), NEVER_DUE);
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        let (done, answered) = mpsc::channel();
        running.power_tx.send(PowerNotice::Suspend(done)).unwrap();
        running.wake.set();
        assert!(answered.recv_timeout(PATIENCE).is_ok());
        let suspended_at = wall_ms();
        thread::sleep(Duration::from_millis(1_500));
        running.power_tx.send(PowerNotice::Resume).unwrap();
        running.wake.set();
        let resumed_at = wall_ms();
        assert!(wait_until(PATIENCE, || {
            lock(&running.status)
                .last_sample_ms
                .is_some_and(|ms| ms >= resumed_at)
        }));
        stop(&running);

        let gaps = history(dir.path()).gaps(i64::MIN, i64::MAX).unwrap();
        let gap = gaps
            .iter()
            .find(|g| g.reason == GapReason::Suspended)
            .expect("the suspend left a gap");
        assert!(gap.start_ms <= suspended_at, "it starts at the suspend");
        assert!(
            gap.end_ms - gap.start_ms >= 1_400,
            "it lasts until the resume"
        );
    }

    #[test]
    fn suspend_is_acknowledged_after_the_batch_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), NEVER_DUE);
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        let (done, answered) = mpsc::channel();
        running.power_tx.send(PowerNotice::Suspend(done)).unwrap();
        running.wake.set();
        assert!(
            answered.recv_timeout(PATIENCE).is_ok(),
            "the suspend was not answered"
        );
        assert_eq!(
            samples(dir.path()).0,
            1,
            "the answer came before the batch was written"
        );
        stop(&running);
    }

    #[test]
    fn a_suspend_notice_wakes_the_sampler_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let running = start_with(
            dir.path(),
            NEVER_DUE,
            Box::new(|| Box::new(QuietOnTheTimer(Quiet))),
        );
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );
        // The sampler is now part way through its second, waiting for the next tick.
        thread::sleep(Duration::from_millis(150));

        let (done, answered) = mpsc::channel();
        let sent = Instant::now();
        running.power_tx.send(PowerNotice::Suspend(done)).unwrap();
        running.wake.set();
        assert!(answered.recv_timeout(PATIENCE).is_ok());
        assert!(
            sent.elapsed() < Duration::from_millis(700),
            "answered after {:?}, the tick is a second long",
            sent.elapsed()
        );
        stop(&running);
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
            bookmark_xml: Some("<BookmarkList/>".to_string()),
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
                bookmark_xml: Some("<BookmarkList/>".to_string()),
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

    /// A sampler driven by hand over a history file in `dir`.
    fn manual_sampler(dir: &std::path::Path) -> Sampler {
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let (wiring, _) = wire(dir, Duration::from_secs(300), quiet_set());
        over(store, wiring)
    }

    /// A sampler over `store`, driven by hand.
    fn over(store: Store, wiring: Wiring) -> Sampler {
        let storage = start_storage(store, &wiring).unwrap();
        Sampler::new(storage, wiring)
    }

    #[test]
    fn the_first_stored_event_batch_requests_a_health_pass() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let (wiring, ties) = wire(dir.path(), Duration::from_secs(300), quiet_set());
        let due = wiring.health_due.clone();
        let mut sampler = over(store, wiring);

        sampler.flush();
        sampler.drain();
        assert!(
            !due.load(Ordering::Acquire),
            "a batch without events asks nothing"
        );

        ties.events_tx
            .send(delivered(
                1_000,
                "disk",
                153,
                EventMeaning::DiskRetry,
                String::new(),
            ))
            .unwrap();
        sampler.take_events();
        sampler.flush();
        sampler.drain();
        assert!(due.load(Ordering::Acquire));

        due.store(false, Ordering::Release);
        ties.events_tx
            .send(delivered(
                2_000,
                "disk",
                153,
                EventMeaning::DiskRetry,
                String::new(),
            ))
            .unwrap();
        sampler.take_events();
        sampler.flush();
        sampler.drain();
        assert!(
            !due.load(Ordering::Acquire),
            "later batches do not ask again"
        );
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
        sampler.drain();

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

    /// Keeps the storage thread busy until the returned flag is set, once it has started.
    fn hold_the_thread(service: &Service) -> Arc<AtomicBool> {
        let release = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let (waited, begun) = (release.clone(), started.clone());
        let hold = Queued {
            epoch: 0,
            job: Job::Run(Box::new(move |_| {
                begun.store(true, Ordering::Release);
                while !waited.load(Ordering::Acquire) {
                    thread::sleep(Duration::from_millis(5));
                }
            })),
        };
        assert!(service.submit(hold).is_ok());
        assert!(wait_until(PATIENCE, || started.load(Ordering::Acquire)));
        release
    }

    #[test]
    fn a_disk_that_answers_after_seconds_holds_up_neither_samples_nor_batches() {
        let dir = tempfile::tempdir().unwrap();
        let script = Arc::new(Mutex::new(Script::default()));
        let (mut wiring, ties) = wire(dir.path(), Duration::from_secs(1), scripted(&script));
        let (asked, release) = (
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        );
        wiring.health = Box::new(Sleepy {
            asked: asked.clone(),
            release: release.clone(),
        });
        let due = wiring.health_due.clone();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let handle = run_sampler(store, wiring).unwrap();
        assert!(
            wait_until(PATIENCE, || {
                lock(&ties.now)
                    .sample
                    .as_ref()
                    .is_some_and(|sample| !sample.disks.is_empty())
            }),
            "no sample with a disk"
        );

        due.store(true, Ordering::Release);
        assert!(wait_until(PATIENCE, || asked.load(Ordering::Acquire)));
        let began = Instant::now();
        let mut seen = lock(&ties.status).last_sample_ms;
        let mut moved = Instant::now();
        let mut longest = Duration::ZERO;
        let mut worst_lag = 0.0f32;
        while began.elapsed() < Duration::from_secs(4) {
            thread::sleep(Duration::from_millis(20));
            let latest = lock(&ties.status).last_sample_ms;
            if latest != seen {
                longest = longest.max(moved.elapsed());
                seen = latest;
                moved = Instant::now();
            }
            if let Some(sample) = lock(&ties.now).sample.as_ref() {
                worst_lag = worst_lag.max(sample.sampler_lag_ms);
            }
        }
        let stored_meanwhile = samples(dir.path()).0;
        release.store(true, Ordering::Release);
        ties.gate.set(GateState::Stopping);
        assert!(handle.wait(Duration::from_secs(5)), "sampler did not stop");

        assert!(
            longest < Duration::from_millis(1_800),
            "the samples stalled for {longest:?} behind the disk"
        );
        assert!(worst_lag < 500.0, "the tick woke {worst_lag} ms late");
        assert!(
            stored_meanwhile >= 1,
            "no batch was stored while the disk read was stuck"
        );
    }

    #[test]
    fn a_write_stuck_on_the_database_does_not_delay_the_samples() {
        let dir = tempfile::tempdir().unwrap();
        let running = start(dir.path(), Duration::from_secs(1));
        assert!(
            wait_until(PATIENCE, || sampled(&running.status)),
            "no sample taken"
        );

        let held = WriterLock::take(dir.path());
        let began = Instant::now();
        let mut seen = lock(&running.status).last_sample_ms;
        let mut moved = Instant::now();
        let mut longest = Duration::ZERO;
        while began.elapsed() < Duration::from_secs(6) {
            thread::sleep(Duration::from_millis(20));
            let latest = lock(&running.status).last_sample_ms;
            if latest != seen {
                longest = longest.max(moved.elapsed());
                seen = latest;
                moved = Instant::now();
            }
        }
        drop(held);
        let stored = wait_until(PATIENCE, || samples(dir.path()).1 >= 1);
        stop(&running);

        assert!(
            longest < Duration::from_millis(1_800),
            "the samples stalled for {longest:?} while the database was busy"
        );
        assert!(
            stored,
            "the batches waiting on the thread were never stored"
        );
    }

    #[test]
    fn what_was_held_before_a_clear_is_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let (wiring, ties) = wire(dir.path(), Duration::from_secs(300), quiet_set());
        let mut sampler = over(store, wiring);
        let ts = wall_ms();
        sampler.batcher.push_sample(quiet(ts));
        sampler.events.push(disk_event(1));
        sampler.gaps.push(Gap {
            start_ms: ts,
            end_ms: ts + 1,
            reason: GapReason::Paused,
        });
        ties.epoch.clear(|| Ok::<_, ()>(())).unwrap();

        sampler.flush_final();
        sampler.drain();

        let store = history(dir.path());
        assert!(store.events(i64::MIN, i64::MAX).unwrap().is_empty());
        assert!(store.gaps(i64::MIN, i64::MAX).unwrap().is_empty());
        assert!(
            store
                .segments(Series::Samples, i64::MIN, i64::MAX)
                .unwrap()
                .is_empty()
        );
        assert_eq!(sampler.seen_epoch, 1);
    }

    #[test]
    fn a_full_queue_keeps_batches_in_the_backlog_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let release = hold_the_thread(&sampler.service);
        let sent = crate::service::QUEUE_BOUND as u64 + 2;
        for n in 1..=sent {
            sampler.events.push(disk_event(n));
            sampler.write(Vec::new());
        }

        assert!(
            !sampler.backlog.is_empty(),
            "the queue did not take them all"
        );
        release.store(true, Ordering::Release);
        sampler.drain();

        assert!(sampler.backlog.is_empty());
        let ids: Vec<u64> = history(dir.path())
            .events(i64::MIN, i64::MAX)
            .unwrap()
            .iter()
            .map(|e| e.record_id)
            .collect();
        assert_eq!(ids, (1..=sent).collect::<Vec<_>>());
    }

    #[test]
    fn a_backlog_beyond_the_cap_loses_the_samples_of_its_oldest_batch() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let release = hold_the_thread(&sampler.service);
        let filler = Batch {
            events: vec![disk_event(1)],
            ..Batch::default()
        };
        for _ in 0..crate::service::QUEUE_BOUND {
            let queued = Queued {
                epoch: 0,
                job: Job::Batch(filler.clone()),
            };
            assert!(sampler.service.submit(queued).is_ok());
        }
        let ts = wall_ms();
        for n in 1..=BATCHES_KEPT as i64 + 1 {
            sampler.batcher.push_sample(quiet(ts + n * 60_000));
            sampler.bookmark_xml = Some(format!("bookmark {n}"));
            sampler.flush_final();
        }

        assert_eq!(sampler.backlog.len(), BATCHES_KEPT);
        let Some(Queued {
            job: Job::Batch(first),
            ..
        }) = sampler.backlog.front()
        else {
            panic!("the backlog holds no batch");
        };
        assert_eq!(
            first.gaps.iter().map(|g| g.reason).collect::<Vec<_>>(),
            [GapReason::NotSaved],
            "the shed batch is recorded as a gap in the next one"
        );
        release.store(true, Ordering::Release);
        sampler.drain();
        let (_, count, _) = samples(dir.path());
        assert_eq!(count, BATCHES_KEPT);
        assert_eq!(
            history(dir.path()).bookmark_xml().unwrap().as_deref(),
            Some(format!("bookmark {}", BATCHES_KEPT + 1).as_str())
        );
    }

    /// A subscription to an event no log holds, so it starts and stays silent.
    fn quiet_subscription(
        tx: mpsc::SyncSender<Delivered>,
    ) -> Result<EventSubscription, CollectError> {
        let query = "<QueryList><Query Id=\"0\"><Select Path=\"Application\">                     *[System[Provider[@Name='VestigiumNoSuchProvider']]]</Select></Query></QueryList>";
        EventSubscription::start_with_query(query, None, vestigium_core::classify, tx)
    }

    #[test]
    fn a_failed_subscription_is_tried_again_every_minute_until_it_starts() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let (tx, _rx) = mpsc::sync_channel(64);
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = attempts.clone();
        sampler.resubscribe = Some(Box::new(move |_| {
            if counted.fetch_add(1, Ordering::AcqRel) < 2 {
                Err(CollectError::Win32(windows::core::Error::from_hresult(
                    windows::core::HRESULT(0x8007_0426_u32 as i32),
                )))
            } else {
                quiet_subscription(tx.clone())
            }
        }));
        let start = sampler.next_subscribe_ms;

        sampler.keep_subscribed(start - 1);
        assert_eq!(
            attempts.load(Ordering::Acquire),
            0,
            "not before the wait is over"
        );

        sampler.keep_subscribed(start);
        assert_eq!(attempts.load(Ordering::Acquire), 1);
        assert!(lock(&sampler.status).last_error.is_some());
        assert!(sampler.subscription.is_none());

        sampler.keep_subscribed(start + SUBSCRIBE_RETRY_MS - 1);
        assert_eq!(attempts.load(Ordering::Acquire), 1);
        sampler.keep_subscribed(start + SUBSCRIBE_RETRY_MS);
        sampler.keep_subscribed(start + 2 * SUBSCRIBE_RETRY_MS);
        assert_eq!(attempts.load(Ordering::Acquire), 3);
        assert!(sampler.subscription.is_some());

        sampler.keep_subscribed(start + 10 * SUBSCRIBE_RETRY_MS);
        assert_eq!(
            attempts.load(Ordering::Acquire),
            3,
            "a live subscription is never started twice"
        );
    }

    #[test]
    fn a_storm_stores_a_capped_batch_and_still_moves_the_bookmark() {
        let dir = tempfile::tempdir().unwrap();
        let (wiring, ties) = wire(dir.path(), Duration::from_secs(300), quiet_set());
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let mut sampler = over(store, wiring);
        for round in 0..4 {
            for n in 0..60 {
                let ts = 1_000 + round * 60 + n;
                let mut storm = delivered(ts, "disk", 153, EventMeaning::DiskRetry, String::new());
                storm.bookmark_xml = Some(format!("<Bookmark at=\"{ts}\"/>"));
                ties.events_tx.send(storm).unwrap();
            }
            sampler.take_events();
        }
        ties.events_tx
            .send(delivered(
                9_000,
                "Display",
                4101,
                EventMeaning::DisplayReset,
                String::new(),
            ))
            .unwrap();
        sampler.take_events();

        let stored = |provider: &str| {
            sampler
                .events
                .iter()
                .filter(|e| e.provider == provider)
                .count()
        };
        assert_eq!(stored("disk"), BATCH_EVENT_EXEMPLARS);
        assert_eq!(stored("Display"), 1, "another event is not crowded out");
        assert_eq!(sampler.bookmark_xml.as_deref(), Some("<BookmarkList/>"));
    }

    #[test]
    fn the_bookmark_follows_the_last_dropped_event() {
        let dir = tempfile::tempdir().unwrap();
        let (wiring, ties) = wire(dir.path(), Duration::from_secs(300), quiet_set());
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let mut sampler = over(store, wiring);
        for round in 0..2 {
            for n in 0..BATCH_EVENT_EXEMPLARS as i64 {
                let at = round * 100 + n;
                let mut storm = delivered(
                    1_000 + at,
                    "disk",
                    153,
                    EventMeaning::DiskRetry,
                    String::new(),
                );
                storm.bookmark_xml = Some(format!("at {at}"));
                ties.events_tx.send(storm).unwrap();
            }
            sampler.take_events();
        }

        assert_eq!(sampler.events.len(), BATCH_EVENT_EXEMPLARS);
        assert_eq!(
            sampler.bookmark_xml,
            Some(format!("at {}", 100 + BATCH_EVENT_EXEMPLARS - 1))
        );
    }

    #[test]
    fn an_event_without_a_bookmark_keeps_the_pending_one() {
        let dir = tempfile::tempdir().unwrap();
        let (wiring, ties) = wire(dir.path(), Duration::from_secs(300), quiet_set());
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let mut sampler = over(store, wiring);
        sampler.bookmark_xml = Some("held".to_string());
        let mut later = delivered(2_000, "disk", 153, EventMeaning::DiskRetry, String::new());
        later.bookmark_xml = None;
        ties.events_tx.send(later).unwrap();

        sampler.take_events();

        assert_eq!(sampler.events.len(), 1);
        assert_eq!(sampler.bookmark_xml.as_deref(), Some("held"));
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
        sampler.drain();
        drop(refusal);
        assert!(
            history(dir.path())
                .occurrences(i64::MIN, i64::MAX)
                .unwrap()
                .is_empty()
        );
        assert!(sampler.service.holds_outputs());

        sampler.keep(Vec::new(), now);
        sampler.drain();
        sampler.keep(Vec::new(), now);
        sampler.drain();

        let stored = history(dir.path()).occurrences(i64::MIN, i64::MAX).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(sampler.blackbox.recent(now).occurrences, 1);
        assert!(!sampler.service.holds_outputs());
    }

    #[test]
    fn occurrences_that_never_store_stop_counting_past_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let (outputs, now) = freeze_outputs(&mut sampler, OUTPUTS_KEPT + 2);
        assert_eq!(outputs.len(), OUTPUTS_KEPT + 2);

        let _refusal = Refusal::of(dir.path(), "occurrence");
        sampler.keep(outputs, now);
        sampler.drain();
        sampler.keep(Vec::new(), now);

        assert_eq!(
            sampler.blackbox.recent(now).occurrences as usize,
            OUTPUTS_KEPT
        );
    }

    #[test]
    fn an_occurrence_behind_a_busy_thread_names_itself_in_the_live_frame_once_stored() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let (outputs, now) = freeze_outputs(&mut sampler, 1);
        let release = hold_the_thread(&sampler.service);
        let began = Instant::now();

        sampler.keep(outputs, now);

        assert!(
            began.elapsed() < Duration::from_millis(500),
            "keeping an occurrence waited for the thread"
        );
        assert!(lock(&sampler.now).latest.is_none());
        release.store(true, Ordering::Release);
        sampler.drain();
        let stored = history(dir.path()).occurrences(i64::MIN, i64::MAX).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            lock(&sampler.now).latest.as_ref().map(|o| o.id),
            Some(stored[0].id)
        );
    }

    #[test]
    fn the_machine_facts_reach_the_history_through_the_thread() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let mut sample = quiet(wall_ms());
        sample.disks = vec![DiskLoad {
            name: "9 Z:".to_string(),
            active_pct: 1.0,
            latency_ms: Some(1.0),
            queue_len: Some(0.0),
            bytes_per_s: Some(0.0),
        }];

        sampler.learn_machine(&sample);
        sampler.drain();

        let machine = history(dir.path()).machine().unwrap().unwrap();
        assert_eq!(machine.disks.len(), 1);
        assert_eq!(machine.disks[0].name, "9 Z:");
    }

    #[test]
    fn a_read_never_waits_for_the_write_lock() {
        let dir = tempfile::tempdir().unwrap();
        let sampler = manual_sampler(dir.path());
        let _writing = WriterLock::take(dir.path());

        let read = sampler
            .history()
            .and_then(|history| history.samples(0, i64::MAX));

        assert!(
            read.unwrap().is_empty(),
            "a reader never waits for the writer"
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
            status.paused_until_ms = Some(interrupt_ms() + 500);
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
    fn hiccups_keep_a_few_per_interval_when_one_interval_bursts() {
        let mut kept = VecDeque::new();
        keep_hiccups(&mut kept, &[tick(0, 20.0), tick(1_000, 30.0)], 1_000);
        let burst: Vec<ProbeTick> = (0..5_000i64)
            .map(|k| tick(300_000 + k, 8.0 + (k % 50) as f32))
            .collect();

        keep_hiccups(&mut kept, &burst, 310_000);

        let early: Vec<i64> = kept.iter().take(2).map(|h| h.ts_ms).collect();
        assert_eq!(early, [0, 1_000], "the earlier minute is not pushed out");
        assert_eq!(kept.len(), 2 + INTERVAL_KEPT);
        assert!(
            kept.iter().skip(2).all(|h| h.lag_ms >= 56.0),
            "the worst stay"
        );
    }

    #[test]
    fn hiccups_arriving_one_by_one_keep_the_worst_of_their_interval() {
        let mut kept = VecDeque::new();
        for k in 0..2_000i64 {
            let lag = if k == 700 { 90.0 } else { 9.0 + (k % 5) as f32 };
            keep_hiccups(&mut kept, &[tick(60_000 + k * 20, lag)], 100_000);
        }
        for k in 0..10i64 {
            keep_hiccups(&mut kept, &[tick(120_000 + k, 12.0)], 130_000);
        }

        assert_eq!(kept.len(), 2 * INTERVAL_KEPT);
        assert!(kept.iter().any(|h| h.lag_ms == 90.0));
        assert!(kept.iter().map(|h| h.ts_ms).is_sorted());
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

    #[test]
    fn the_prune_is_sent_once_an_hour() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let mut writer = history(dir.path());
        let old_gap = || Batch {
            gaps: vec![Gap {
                start_ms: 1_000,
                end_ms: 2_000,
                reason: GapReason::Paused,
            }],
            ..Batch::default()
        };
        let now = 400 * DAY_MS;
        writer.append_batch(&old_gap()).unwrap();

        assert!(sampler.prune_if_due(now));
        sampler.drain();
        assert!(writer.gaps(i64::MIN, i64::MAX).unwrap().is_empty());

        writer.append_batch(&old_gap()).unwrap();
        assert!(!sampler.prune_if_due(now + HOUR_MS - 1));
        sampler.drain();
        assert_eq!(writer.gaps(i64::MIN, i64::MAX).unwrap().len(), 1);
        assert!(sampler.prune_if_due(now + HOUR_MS));
        sampler.drain();
        assert!(writer.gaps(i64::MIN, i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn a_prune_behind_a_stuck_thread_does_not_hold_up_the_tick() {
        let dir = tempfile::tempdir().unwrap();
        let mut sampler = manual_sampler(dir.path());
        let release = hold_the_thread(&sampler.service);
        let began = Instant::now();

        assert!(sampler.prune_if_due(wall_ms()));
        sampler.check_dumps(wall_ms());
        sampler.write(Vec::new());

        assert!(
            began.elapsed() < Duration::from_millis(500),
            "the prune and the inventory waited for the thread"
        );
        release.store(true, Ordering::Release);
        sampler.drain();
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
        let (probe_tx, probe_rx) = mpsc::sync_channel(64);
        let (_power_tx, power_rx) = mpsc::channel();
        let (_events_tx, events_rx) = mpsc::sync_channel(64);
        let wiring = Wiring {
            flush: Duration::from_secs(1),
            timing: QUICK,
            gate: shared.gate.clone(),
            probe_rx,
            power_rx,
            wake: Arc::new(Wake::new().unwrap()),
            events_rx,
            subscription: None,
            resubscribe: None,
            sensitive: shared.sensitive.clone(),
            status: shared.status.clone(),
            recent: shared.recent.clone(),
            now: shared.now.clone(),
            readings: shared.readings.clone(),
            programs: shared.programs.clone(),
            pressure: shared.pressure.clone(),
            hiccups: shared.hiccups.clone(),
            settings: shared.settings.clone(),
            epoch: shared.epoch.clone(),
            health_due: shared.health_due.clone(),
            ring_path: Some(dir.path().join("blackbox.ring")),
            source_set: quiet_set(),
            health: Box::new(NoHealth),
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
        assert!(
            wait_until(PATIENCE, || sampled.load(Ordering::Acquire) > 0),
            "the sampler never took a sample"
        );
        let before = (
            count(&pipe_turns),
            count(&tray_turns),
            sampled.load(Ordering::Acquire),
        );
        late_tick();
        late_tick();
        // Each loop must advance past where it was, however long a loaded machine takes.
        let progressed = wait_until(PATIENCE, || {
            count(&pipe_turns) > before.0
                && count(&tray_turns) > before.1
                && sampled.load(Ordering::Acquire) > before.2
        });
        let after = (count(&pipe_turns), count(&tray_turns));

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
        assert!(progressed, "the sampler stopped taking samples");
    }
}
