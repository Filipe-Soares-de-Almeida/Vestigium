use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::mem;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use vestigium_collect::{
    CollectError, CpuSampler, Delivered, EventSubscription, HighResTimer, PdhSampler,
    ProcessSnapshotter, disk_media, hung_pids, logical_cpus, memory_info, physical_drive,
    unbiased_ms, wall_ms,
};
use vestigium_core::{
    Blackbox, DiskInfo, EpisodeKind, Gap, GapReason, GateState, HungWindow, MachineInfo, Media,
    Notable, OccurrenceSummary, Output, ProbeTick, ProcessCounters, ProcessTop, RecentState,
    RunGate, Sample, Settings, Severity, Timing, WinEvent, top_processes,
};
use vestigium_ipc::{AgentStatus, NowFrame};
use vestigium_store::{Batch, Batcher, Segment, Store};
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

use crate::app::lock;
use crate::power::PowerEvent;
use crate::sensitive::SensitiveMode;
use crate::shutdown;

const TICK_MS: f64 = 1000.0;
/// Seconds between two stored process rankings.
const TOP_EVERY: usize = 5;
/// Seconds between two checks for windows that stopped responding.
const HANG_EVERY: usize = 2;
const TOP_PER_METRIC: usize = 5;
const SNAPSHOT_BUFFER: usize = 512 * 1024;
/// Ticks after a start or a resume before idle pages are handed back to Windows.
const TRIM_AFTER_TICKS: usize = 10;
/// A stop shorter than this is a restart, not a gap.
const STOPPED_GAP_MIN_MS: i64 = 5_000;
const HOUR_MS: i64 = 60 * 60_000;
const DAY_MS: i64 = 24 * HOUR_MS;

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
    /// Retention is read from here at every prune, so a change applies on the next one.
    pub settings: Arc<Mutex<Settings>>,
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
            sampler.leave(GapReason::Stopped);
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
    settings: Arc<Mutex<Settings>>,
    /// Events and the bookmark that covers them, waiting for the next batch.
    events: Vec<WinEvent>,
    bookmark_xml: Option<String>,
    gaps: Vec<Gap>,
    /// When watching stopped and why, while paused or suspended.
    away_since: Option<(i64, GapReason)>,
    pruned_hour: Option<i64>,
    snapshots: VecDeque<(f64, Vec<ProcessCounters>)>,
    /// GPU use per process at the latest sample, for the next ranking.
    gpu_by_pid: HashMap<u32, f32>,
    logical_cpus: u32,
}

/// The data sources a tick reads, opened once on the sampler thread.
struct Sources {
    cpu: CpuSampler,
    pdh: PdhSampler,
    processes: ProcessSnapshotter,
}

impl Sources {
    /// Takes the first readings, which only set the baseline for rates.
    fn prime(&mut self) {
        let _ = self.cpu.sample();
        let _ = self.pdh.sample();
    }

    fn reset(&mut self) {
        self.cpu.reset();
        self.pdh.reset();
    }
}

impl Sampler {
    fn new(store: Store, wiring: Wiring) -> Sampler {
        let now = wall_ms();
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
            settings: wiring.settings,
            events: Vec::new(),
            bookmark_xml: None,
            gaps: Vec::new(),
            away_since: None,
            pruned_hour: None,
            snapshots: VecDeque::with_capacity(TOP_EVERY + 1),
            gpu_by_pid: HashMap::new(),
            logical_cpus: logical_cpus(),
            store,
        };
        sampler.recall_last_day(now);
        if let Ok(Some(last)) = sampler.store.last_sample_end_ms()
            && now - last > STOPPED_GAP_MIN_MS
        {
            sampler.away_since = Some((last, GapReason::Stopped));
            sampler.come_back(now);
        }
        sampler
    }

    /// Counts what the history already holds for the last 24 h, so the tray survives a restart.
    fn recall_last_day(&mut self, now: i64) {
        if let Ok(occurrences) = self.store.occurrences(now - DAY_MS, now) {
            lock(&self.now).latest = occurrences.last().cloned();
            for o in occurrences {
                let severe = o.severity == Severity::Severe;
                self.blackbox
                    .remember(o.end_ms, Notable::Occurrence { severe });
            }
        }
        if let Ok(episodes) = self.store.episodes(now - DAY_MS, now) {
            for e in episodes.iter().filter(|e| !is_hint(&e.kind)) {
                self.blackbox.remember(e.end_ms, Notable::Episode);
            }
        }
        self.publish_recent(now);
    }

    /// Ticks once a second until the gate says stop; pausing writes everything and blocks.
    /// Time runs on the unbiased clock, so a sleep of the whole system is not a late tick.
    fn run(&mut self, gate: &RunGate) {
        let opened = PdhSampler::open().and_then(|pdh| HighResTimer::new().map(|t| (pdh, t)));
        let (pdh, timer) = match opened {
            Ok(parts) => parts,
            Err(err) => return self.report(&err),
        };
        let mut sources = Sources {
            cpu: CpuSampler::new(),
            pdh,
            processes: ProcessSnapshotter::with_capacity(SNAPSHOT_BUFFER),
        };
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
                    if gate.wait_while_paused() == GateState::Stopping {
                        return;
                    }
                    self.probe_rx.try_iter().for_each(drop);
                    self.come_back(wall_ms());
                    sources.prime();
                    next_tick = unbiased_ms() + TICK_MS;
                    trim_at = Some(ticks + TRIM_AFTER_TICKS);
                    continue;
                }
                GateState::Running => {}
            }

            let due_ms = (next_tick - unbiased_ms()).max(0.0);
            if let Err(err) = timer.wait(Duration::from_secs_f64(due_ms / 1000.0)) {
                return self.report(&err);
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
            if self.sensitive.expire(wall_ms()) {
                shutdown::wake();
            }

            ticks += 1;
            if let Err(err) = self.tick(&mut sources, woke, sampler_lag_ms, ticks) {
                self.report(&err);
            }
            if trim_at == Some(ticks) {
                trim_at = None;
                release_idle_pages();
            }
        }
    }

    fn tick(
        &mut self,
        sources: &mut Sources,
        woke: f64,
        sampler_lag_ms: f32,
        ticks: usize,
    ) -> Result<(), CollectError> {
        let ts_ms = wall_ms();
        self.blackbox.set_sensitive(self.sensitive.is_on());
        let mut probe_max_lag_ms: f32 = 0.0;
        for tick in self.probe_rx.try_iter() {
            probe_max_lag_ms = probe_max_lag_ms.max(tick.lag_ms);
            self.blackbox.push_probe(tick);
        }
        let memory = memory_info()?;
        let (cpu, pdh) = (sources.cpu.sample()?, sources.pdh.sample()?);
        if let (Some(cpu), Some(pdh)) = (cpu, pdh) {
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
            lock(&self.now).sample = Some(sample.clone());
            self.blackbox.push_sample(sample.clone());
            self.batcher.push_sample(sample);
            lock(&self.status).last_sample_ms = Some(ts_ms);
        }

        if self.snapshots.len() > TOP_EVERY {
            self.snapshots.pop_front();
        }
        self.snapshots
            .push_back((woke, sources.processes.snapshot()?));
        if ticks.is_multiple_of(HANG_EVERY) {
            self.check_hangs(ts_ms);
        }
        if ticks.is_multiple_of(TOP_EVERY)
            && self.snapshots.len() > TOP_EVERY
            && let (Some((then, old)), Some((now, new))) =
                (self.snapshots.front(), self.snapshots.back())
        {
            let top = ProcessTop {
                ts_ms,
                entries: top_processes(
                    old,
                    new,
                    (now - then).max(0.0) as u64,
                    self.logical_cpus,
                    TOP_PER_METRIC,
                    &self.gpu_by_pid,
                ),
            };
            lock(&self.now).top = Some(top.clone());
            self.blackbox.push_top(top.clone());
            self.batcher.push_processes(top);
        }

        self.take_events();
        let outputs = self.blackbox.poll(ts_ms);
        self.keep(outputs, ts_ms);
        if self.batcher.due(ts_ms) {
            self.flush();
        }
        if prune_due(self.pruned_hour, ts_ms) {
            self.pruned_hour = Some(ts_ms.div_euclid(HOUR_MS));
            let retention = lock(&self.settings).retention;
            if let Err(err) = self.store.prune(ts_ms, &retention) {
                self.report(&err);
            }
        }
        Ok(())
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
        while let Ok(delivered) = self.events_rx.try_recv() {
            self.blackbox.push_event(delivered.event.clone());
            self.events.push(delivered.event);
            self.bookmark_xml = Some(delivered.bookmark_xml);
        }
    }

    /// Stores occurrences and episodes right away; gaps wait for the batch.
    fn keep(&mut self, outputs: Vec<Output>, now: i64) {
        for output in outputs {
            let stored = match output {
                Output::Occurrence(occurrence) => {
                    self.store.insert_occurrence(&occurrence).map(|id| {
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
                Output::Episode(episode) => self.store.insert_episode(&episode).map(drop),
                Output::Gap(gap) => {
                    self.gaps.push(gap);
                    Ok(())
                }
            };
            if let Err(err) = stored {
                self.report(&err);
            }
        }
        self.publish_recent(now);
    }

    /// Hands the tray the counts of the last 24 h, and wakes it when they changed.
    fn publish_recent(&self, now: i64) {
        let recent = self.blackbox.recent(now);
        let mut shown = lock(&self.recent);
        if *shown != recent {
            *shown = recent;
            drop(shown);
            shutdown::wake();
        }
    }

    /// Stores the machine facts once the disk counters name the disks.
    fn learn_machine(&mut self, sample: &Sample) {
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
    fn leave(&mut self, reason: GapReason) {
        let now = wall_ms();
        self.take_events();
        let outputs = self.blackbox.drain(now);
        self.keep(outputs, now);
        self.flush_final();
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

    /// Writes segments, events with their bookmark, and gaps in one transaction.
    fn write(&mut self, segments: Vec<Segment>) {
        let batch = Batch {
            segments,
            events: mem::take(&mut self.events),
            bookmark_xml: self.bookmark_xml.take(),
            gaps: mem::take(&mut self.gaps),
        };
        if batch.is_empty() {
            return;
        }
        if let Err(err) = self.store.append_batch(&batch) {
            self.report(&err);
        }
    }

    fn report(&self, err: &dyn std::fmt::Display) {
        lock(&self.status).last_error = Some(err.to_string());
    }
}

fn is_hint(kind: &EpisodeKind) -> bool {
    matches!(
        kind,
        EpisodeKind::DpcIsrHint { .. } | EpisodeKind::DiskLatencyHint { .. }
    )
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
        Channel, DEFAULT_PROBE_PERIOD, EventMeaning, OccurrenceKind, SENSITIVE_FOR, Sample,
    };
    use vestigium_store::{Series, decode, encode};

    use super::*;

    struct Running {
        gate: Arc<RunGate>,
        handle: SamplerHandle,
        status: Arc<Mutex<AgentStatus>>,
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
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let gate = RunGate::new();
        let (probe_tx, probe_rx) = mpsc::channel();
        let (power_tx, power_rx) = mpsc::channel();
        let (events_tx, events_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(AgentStatus::default()));
        let sensitive = SensitiveMode::new(ProbeConfig::new(DEFAULT_PROBE_PERIOD), SENSITIVE_FOR);
        let handle = run_sampler(
            store,
            Wiring {
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
                now: Arc::new(Mutex::new(NowFrame::default())),
                settings: Arc::new(Mutex::new(Settings::default())),
            },
        )
        .unwrap();
        Running {
            gate,
            handle,
            status,
            probe_tx,
            power_tx,
            events_tx,
        }
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

    #[test]
    fn prune_runs_when_the_hour_changes() {
        let hour = 5;
        let start = hour * HOUR_MS;

        assert!(prune_due(None, start));
        assert!(!prune_due(Some(hour), start));
        assert!(!prune_due(Some(hour), start + HOUR_MS - 1));
        assert!(prune_due(Some(hour), start + HOUR_MS));
    }
}
