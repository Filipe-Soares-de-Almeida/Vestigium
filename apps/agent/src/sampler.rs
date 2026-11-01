use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use vestigium_collect::{
    CollectError, CpuSampler, HighResTimer, PdhSampler, ProcessSnapshotter, logical_cpus,
    memory_info, unbiased_ms, wall_ms,
};
use vestigium_core::{
    GateState, ProbeTick, ProcessCounters, ProcessTop, Ring, RunGate, Sample, top_processes,
};
use vestigium_ipc::AgentStatus;
use vestigium_store::{Batcher, Segment, Store};
use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};

use crate::app::lock;
use crate::power::PowerEvent;
use crate::sensitive::SensitiveMode;
use crate::shutdown;

const TICK_MS: f64 = 1000.0;
/// Seconds between two stored process rankings.
const TOP_EVERY: usize = 5;
const TOP_PER_METRIC: usize = 5;
const RING_SAMPLES: usize = 600;
const RING_TOPS: usize = 600 / TOP_EVERY;
const SNAPSHOT_BUFFER: usize = 512 * 1024;
/// Ticks after a start or a resume before idle pages are handed back to Windows.
const TRIM_AFTER_TICKS: usize = 10;

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

/// Starts the 1 Hz sampler, which writes into `store` in batches of `flush`.
pub fn run_sampler(
    store: Store,
    flush: Duration,
    gate: Arc<RunGate>,
    probe_rx: Receiver<ProbeTick>,
    power_rx: Receiver<PowerEvent>,
    sensitive: Arc<SensitiveMode>,
    status: Arc<Mutex<AgentStatus>>,
) -> io::Result<SamplerHandle> {
    let (done_tx, done_rx) = mpsc::channel();
    thread::Builder::new()
        .name("sampler".to_string())
        .spawn(move || {
            let mut sampler = Sampler::new(store, flush, probe_rx, power_rx, sensitive, status);
            sampler.run(&gate);
            sampler.flush_final();
            let _ = done_tx.send(());
        })?;
    Ok(SamplerHandle {
        done: Mutex::new(done_rx),
    })
}

struct Sampler {
    store: Store,
    batcher: Batcher,
    probe_rx: Receiver<ProbeTick>,
    power_rx: Receiver<PowerEvent>,
    sensitive: Arc<SensitiveMode>,
    status: Arc<Mutex<AgentStatus>>,
    samples: Ring<Sample>,
    tops: Ring<ProcessTop>,
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
    fn new(
        store: Store,
        flush: Duration,
        probe_rx: Receiver<ProbeTick>,
        power_rx: Receiver<PowerEvent>,
        sensitive: Arc<SensitiveMode>,
        status: Arc<Mutex<AgentStatus>>,
    ) -> Sampler {
        Sampler {
            store,
            batcher: Batcher::new(flush),
            probe_rx,
            power_rx,
            sensitive,
            status,
            samples: Ring::with_capacity(RING_SAMPLES),
            tops: Ring::with_capacity(RING_TOPS),
            snapshots: VecDeque::with_capacity(TOP_EVERY + 1),
            gpu_by_pid: HashMap::new(),
            logical_cpus: logical_cpus(),
        }
    }

    /// Ticks once a second until the gate says stop; pausing writes the batch and blocks.
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
                    self.flush_final();
                    sources.reset();
                    self.snapshots.clear();
                    if gate.wait_while_paused() == GateState::Stopping {
                        return;
                    }
                    self.probe_rx.try_iter().for_each(drop);
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
        let probe_max_lag_ms = self
            .probe_rx
            .try_iter()
            .fold(0.0, |max: f32, tick| max.max(tick.lag_ms));
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
            self.samples.push(sample.clone());
            self.batcher.push_sample(sample);
            lock(&self.status).last_sample_ms = Some(ts_ms);
        }

        if self.snapshots.len() > TOP_EVERY {
            self.snapshots.pop_front();
        }
        self.snapshots
            .push_back((woke, sources.processes.snapshot()?));
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
            self.tops.push(top.clone());
            self.batcher.push_processes(top);
        }

        if self.batcher.due(ts_ms) {
            self.flush();
        }
        Ok(())
    }

    /// Handles pending suspend and resume notices; true when a resume restarted the sources.
    fn on_power(&mut self, sources: &mut Sources) -> bool {
        let mut resumed = false;
        while let Ok(event) = self.power_rx.try_recv() {
            match event {
                PowerEvent::Suspend => self.flush_final(),
                PowerEvent::Resume => {
                    // Probe ticks and rates measured across the suspension are dropped.
                    self.probe_rx.try_iter().for_each(drop);
                    sources.reset();
                    sources.prime();
                    self.snapshots.clear();
                    resumed = true;
                }
            }
        }
        resumed
    }

    /// Writes whatever is buffered except the minute still open.
    fn flush(&mut self) {
        let segments = self.batcher.take();
        self.write(&segments);
    }

    /// Writes everything buffered, the open minute included, before a pause or a stop.
    fn flush_final(&mut self) {
        let segments = self.batcher.take_final();
        self.write(&segments);
    }

    fn write(&mut self, segments: &[Segment]) {
        if segments.is_empty() {
            return;
        }
        if let Err(err) = self.store.append(segments) {
            self.report(&err);
        }
    }

    fn report(&self, err: &dyn std::fmt::Display) {
        lock(&self.status).last_error = Some(err.to_string());
    }
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
    use vestigium_core::{DEFAULT_PROBE_PERIOD, SENSITIVE_FOR, Sample};
    use vestigium_store::{Series, decode};

    use super::*;

    struct Running {
        gate: Arc<RunGate>,
        handle: SamplerHandle,
        status: Arc<Mutex<AgentStatus>>,
        power_tx: mpsc::Sender<PowerEvent>,
    }

    fn start(dir: &std::path::Path, flush: Duration) -> Running {
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let gate = RunGate::new();
        let (_probe_tx, probe_rx) = mpsc::channel();
        let (power_tx, power_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(AgentStatus::default()));
        let sensitive = SensitiveMode::new(ProbeConfig::new(DEFAULT_PROBE_PERIOD), SENSITIVE_FOR);
        let handle = run_sampler(
            store,
            flush,
            gate.clone(),
            probe_rx,
            power_rx,
            sensitive,
            status.clone(),
        )
        .unwrap();
        Running {
            gate,
            handle,
            status,
            power_tx,
        }
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
}
