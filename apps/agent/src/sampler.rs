use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use vestigium_collect::{
    CollectError, HighResTimer, PdhSampler, ProcessSnapshotter, logical_cpus, memory_info,
    monotonic_ms,
};
use vestigium_core::{GateState, ProcessCounters, ProcessTop, RunGate, Sample, top_processes};
use vestigium_ipc::AgentStatus;
use vestigium_store::{Batcher, Ring, Store};

use crate::app::{lock, now_ms};

const TICK_MS: f64 = 1000.0;
/// Seconds between two stored process rankings.
const TOP_EVERY: usize = 5;
const TOP_PER_METRIC: usize = 5;
const RING_SAMPLES: usize = 600;
const RING_TOPS: usize = 600 / TOP_EVERY;
const SNAPSHOT_BUFFER: usize = 512 * 1024;

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
    probe_rx: Receiver<f32>,
    status: Arc<Mutex<AgentStatus>>,
) -> io::Result<SamplerHandle> {
    let (done_tx, done_rx) = mpsc::channel();
    thread::Builder::new()
        .name("sampler".to_string())
        .spawn(move || {
            let mut sampler = Sampler::new(store, flush, probe_rx, status);
            sampler.run(&gate);
            sampler.flush();
            let _ = done_tx.send(());
        })?;
    Ok(SamplerHandle {
        done: Mutex::new(done_rx),
    })
}

struct Sampler {
    store: Store,
    batcher: Batcher,
    probe_rx: Receiver<f32>,
    status: Arc<Mutex<AgentStatus>>,
    samples: Ring<Sample>,
    tops: Ring<ProcessTop>,
    snapshots: VecDeque<(f64, Vec<ProcessCounters>)>,
    logical_cpus: u32,
}

impl Sampler {
    fn new(
        store: Store,
        flush: Duration,
        probe_rx: Receiver<f32>,
        status: Arc<Mutex<AgentStatus>>,
    ) -> Sampler {
        Sampler {
            store,
            batcher: Batcher::new(flush),
            probe_rx,
            status,
            samples: Ring::with_capacity(RING_SAMPLES),
            tops: Ring::with_capacity(RING_TOPS),
            snapshots: VecDeque::with_capacity(TOP_EVERY + 1),
            logical_cpus: logical_cpus(),
        }
    }

    /// Ticks once a second until the gate says stop; pausing writes the batch and blocks.
    fn run(&mut self, gate: &RunGate) {
        let opened = PdhSampler::open().and_then(|pdh| HighResTimer::new().map(|t| (pdh, t)));
        let (mut pdh, timer) = match opened {
            Ok(parts) => parts,
            Err(err) => return self.report(&err),
        };
        let mut snapshotter = ProcessSnapshotter::with_capacity(SNAPSHOT_BUFFER);
        let _ = pdh.sample();
        let mut next_tick = monotonic_ms() + TICK_MS;
        let mut ticks = 0usize;
        loop {
            match gate.state() {
                GateState::Stopping => return,
                GateState::Paused => {
                    self.flush();
                    pdh.reset();
                    self.snapshots.clear();
                    if gate.wait_while_paused() == GateState::Stopping {
                        return;
                    }
                    self.probe_rx.try_iter().for_each(drop);
                    let _ = pdh.sample();
                    next_tick = monotonic_ms() + TICK_MS;
                    continue;
                }
                GateState::Running => {}
            }

            let due_ms = (next_tick - monotonic_ms()).max(0.0);
            if let Err(err) = timer.wait(Duration::from_secs_f64(due_ms / 1000.0)) {
                return self.report(&err);
            }
            let woke = monotonic_ms();
            let sampler_lag_ms = (woke - next_tick).max(0.0) as f32;
            // After a long stall, start a fresh schedule instead of catching up.
            next_tick = (next_tick + TICK_MS).max(woke + TICK_MS / 2.0);
            if gate.state() != GateState::Running {
                continue;
            }

            ticks += 1;
            if let Err(err) = self.tick(&mut pdh, &mut snapshotter, woke, sampler_lag_ms, ticks) {
                self.report(&err);
            }
        }
    }

    fn tick(
        &mut self,
        pdh: &mut PdhSampler,
        snapshotter: &mut ProcessSnapshotter,
        woke: f64,
        sampler_lag_ms: f32,
        ticks: usize,
    ) -> Result<(), CollectError> {
        let ts_ms = now_ms();
        let probe_max_lag_ms = self.probe_rx.try_iter().fold(0.0, f32::max);
        let memory = memory_info()?;
        if let Some(pdh) = pdh.sample()? {
            let sample = Sample {
                ts_ms,
                cpu_pct: pdh.cpu_pct,
                dpc_pct: pdh.dpc_pct,
                isr_pct: pdh.isr_pct,
                perf_limit_pct: pdh.perf_limit_pct,
                proc_perf_pct: pdh.proc_perf_pct,
                mem_avail_bytes: memory.avail_bytes,
                commit_bytes: memory.commit_bytes,
                commit_limit_bytes: memory.commit_limit_bytes,
                hard_faults_per_s: pdh.hard_faults_per_s,
                probe_max_lag_ms,
                sampler_lag_ms,
                cores: pdh.cores,
                disks: pdh.disks,
            };
            self.samples.push(sample.clone());
            self.batcher.push_sample(sample);
        }

        if self.snapshots.len() > TOP_EVERY {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back((woke, snapshotter.snapshot()?));
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
                ),
            };
            self.tops.push(top.clone());
            self.batcher.push_processes(top);
        }

        let mut status = lock(&self.status);
        status.last_sample_ms = Some(ts_ms);
        drop(status);
        if self.batcher.due(ts_ms) {
            self.flush();
        }
        Ok(())
    }

    /// Writes whatever is buffered.
    fn flush(&mut self) {
        let segments = self.batcher.take();
        if segments.is_empty() {
            return;
        }
        if let Err(err) = self.store.append(&segments) {
            self.report(&err);
        }
    }

    fn report(&self, err: &dyn std::fmt::Display) {
        lock(&self.status).last_error = Some(err.to_string());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use vestigium_core::Sample;
    use vestigium_store::{Series, decode};

    use super::*;

    fn start(
        dir: &std::path::Path,
        flush: Duration,
    ) -> (Arc<RunGate>, SamplerHandle, Arc<Mutex<AgentStatus>>) {
        let store = Store::open(&dir.join("vestigium.db")).unwrap();
        let gate = RunGate::new();
        let (_probe_tx, probe_rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(AgentStatus::default()));
        let handle = run_sampler(store, flush, gate.clone(), probe_rx, status.clone()).unwrap();
        (gate, handle, status)
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

    #[test]
    fn sampler_writes_and_flushes_on_stop() {
        let dir = tempfile::tempdir().unwrap();
        let (gate, handle, status) = start(dir.path(), Duration::from_secs(1));

        thread::sleep(Duration::from_millis(3_500));
        gate.set(GateState::Stopping);
        let stopped_at = now_ms();
        assert!(handle.wait(Duration::from_secs(3)), "sampler did not stop");

        let (segments, count, last_end) = samples(dir.path());
        assert!(segments >= 2, "{segments} segments");
        assert!(count >= 3, "{count} samples");
        assert!(stopped_at - last_end.unwrap() <= 1_500);
        assert_eq!(lock(&status).last_error, None);
    }

    #[test]
    fn paused_sampler_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (gate, handle, _status) = start(dir.path(), Duration::from_secs(1));
        thread::sleep(Duration::from_millis(1_500));

        gate.set(GateState::Paused);
        thread::sleep(Duration::from_millis(1_200));
        let before = samples(dir.path()).1;
        thread::sleep(Duration::from_millis(3_000));
        let after = samples(dir.path()).1;
        gate.set(GateState::Stopping);
        assert!(handle.wait(Duration::from_secs(3)));

        assert!(before >= 1);
        assert_eq!(before, after);
    }
}
