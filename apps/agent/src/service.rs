use std::collections::VecDeque;
use std::io;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use vestigium_core::{Episode, Gap, GapReason, MachineInfo, Occurrence, OccurrenceSummary, Output};
use vestigium_ipc::{AgentStatus, NowFrame};
use vestigium_store::{Batch, Store, StoreError};

use crate::app::{lock, panic_text};

/// Jobs the queue holds before a sender has to keep its work to itself.
pub const QUEUE_BOUND: usize = 32;
/// Batches kept for another try after a failed write, on the thread and in a sender's backlog;
/// past this the oldest loses its samples.
pub const BATCHES_KEPT: usize = 3;
/// Occurrences and episodes kept for another try after a failed write; each one is large.
pub const OUTPUTS_KEPT: usize = 5;
/// Stack of the storage thread: it blocks on its queue and holds no deep call chain.
const STACK_BYTES: usize = 1 << 20;
/// How long a sender waits between two tries to put a job in a full queue.
const RETRY_EVERY: Duration = Duration::from_millis(2);

/// Counts the times the history was cleared, so a write made before a clear is told from one made
/// after it. A clear and a write never overlap: both hold the same guard.
#[derive(Debug, Default)]
pub struct Epoch {
    guard: Mutex<()>,
    value: AtomicU64,
}

impl Epoch {
    /// The number of clears so far; reading it never waits for a clear or a write.
    pub fn current(&self) -> u64 {
        self.value.load(Ordering::Acquire)
    }

    /// Runs `clear` while no write goes through; a clear that succeeded starts a new epoch.
    pub fn clear<T, E>(&self, clear: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        let _held = lock(&self.guard);
        let cleared = clear()?;
        self.value.fetch_add(1, Ordering::AcqRel);
        Ok(cleared)
    }

    fn hold(&self) -> MutexGuard<'_, ()> {
        lock(&self.guard)
    }
}

/// One piece of work for the storage thread.
pub enum Job {
    /// Samples, events with their bookmark, and gaps, written in one transaction.
    Batch(Batch),
    /// A freeze, a stall or a fall; the live frame names it once it is stored.
    Occurrence(Box<Occurrence>),
    Episode(Episode),
    /// The machine facts the disk counters made known.
    Machine(MachineInfo),
    /// Tries again what failed to write: batches, occurrences and episodes.
    Retry,
    /// Answers once every job before it has run.
    Flush(Sender<()>),
    /// Answers once every job before it has run, then ends the thread.
    Stop(Sender<()>),
    /// Runs a closure on the store, so a test can hold the thread or look inside it.
    #[cfg(test)]
    Run(Box<dyn FnOnce(&mut Store) + Send>),
}

/// A job and the epoch of the history it was made for.
pub struct Queued {
    pub epoch: u64,
    pub job: Job,
}

/// What the storage thread shares with the rest of the agent.
pub struct Links {
    /// Receives the text of an error.
    pub status: Arc<Mutex<AgentStatus>>,
    pub epoch: Arc<Epoch>,
    /// Set after the first stored batch that holds events, to ask for a health pass.
    pub health_due: Arc<AtomicBool>,
    /// The live frame, which names the newest occurrence stored.
    pub now: Arc<Mutex<NowFrame>>,
    /// Receives the occurrences and episodes given up after too many failed writes.
    pub dropped: Sender<Output>,
}

/// The handle to the storage thread, which owns the history's write connection.
#[derive(Clone)]
pub struct Service {
    queue: SyncSender<Queued>,
    epoch: Arc<Epoch>,
    /// True while batches whose write failed are waiting for another try.
    retrying: Arc<AtomicBool>,
    /// True while occurrences or episodes whose write failed are waiting for another try.
    holding: Arc<AtomicBool>,
}

impl Service {
    /// Starts the thread; it ends when it gets a `Stop`, or when every handle is dropped.
    pub fn start(store: Store, links: Links) -> io::Result<Service> {
        let (queue, jobs) = mpsc::sync_channel(QUEUE_BOUND);
        let retrying = Arc::new(AtomicBool::new(false));
        let holding = Arc::new(AtomicBool::new(false));
        let service = Service {
            queue,
            epoch: links.epoch.clone(),
            retrying: retrying.clone(),
            holding: holding.clone(),
        };
        let worker = Worker {
            store,
            links,
            retrying,
            holding,
            waiting: VecDeque::new(),
            unstored: VecDeque::new(),
            event_pass_pending: true,
            seen_epoch: 0,
        };
        thread::Builder::new()
            .name("storage".to_string())
            .stack_size(STACK_BYTES)
            .spawn(move || worker.run(jobs))?;
        Ok(service)
    }

    /// Puts a job in the queue at once; a full queue, or a thread that ended, hands it back.
    pub fn submit(&self, queued: Queued) -> Result<(), Queued> {
        match self.queue.try_send(queued) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(back) | TrySendError::Disconnected(back)) => Err(back),
        }
    }

    /// Whether batches that failed to write wait on the thread for another try.
    pub fn is_retrying(&self) -> bool {
        self.retrying.load(Ordering::Acquire)
    }

    /// Whether occurrences or episodes that failed to write wait on the thread for another try.
    pub fn holds_outputs(&self) -> bool {
        self.holding.load(Ordering::Acquire)
    }

    /// The epoch a job made now belongs to.
    pub fn epoch(&self) -> u64 {
        self.epoch.current()
    }

    /// Waits until everything queued so far has run, for at most `timeout`.
    pub fn flush(&self, timeout: Duration) -> bool {
        self.ask(timeout, Job::Flush)
    }

    /// Waits until everything queued so far has run and the thread has ended, for at most `timeout`.
    pub fn stop(&self, timeout: Duration) -> bool {
        self.ask(timeout, Job::Stop)
    }

    fn ask(&self, timeout: Duration, job: impl FnOnce(Sender<()>) -> Job) -> bool {
        let deadline = Instant::now() + timeout;
        let (ack, answered) = mpsc::channel();
        let mut queued = Queued {
            epoch: self.epoch(),
            job: job(ack),
        };
        loop {
            match self.submit(queued) {
                Ok(()) => break,
                Err(back) if Instant::now() < deadline => {
                    queued = back;
                    thread::sleep(RETRY_EVERY);
                }
                Err(_) => return false,
            }
        }
        answered
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .is_ok()
    }
}

struct Worker {
    store: Store,
    links: Links,
    retrying: Arc<AtomicBool>,
    holding: Arc<AtomicBool>,
    /// Batches whose write failed, oldest first, tried again with the next batch.
    waiting: VecDeque<Batch>,
    /// Occurrences and episodes whose write failed, oldest first.
    unstored: VecDeque<Output>,
    /// Set until the first stored batch with events has asked for a health pass.
    event_pass_pending: bool,
    /// The newest epoch a job has come from.
    seen_epoch: u64,
}

impl Worker {
    fn run(mut self, jobs: Receiver<Queued>) {
        while let Ok(Queued { epoch, job }) = jobs.recv() {
            let handled = panic::catch_unwind(AssertUnwindSafe(|| self.handle(epoch, job)));
            match handled {
                Ok(true) => {}
                Ok(false) => return,
                Err(payload) => self.report(&format!(
                    "a history write stopped: {}",
                    panic_text(payload.as_ref())
                )),
            }
        }
    }

    /// Runs one job; false once the thread is to end.
    fn handle(&mut self, epoch: u64, job: Job) -> bool {
        match job {
            Job::Flush(answer) => {
                let _ = answer.send(());
                return true;
            }
            Job::Stop(answer) => {
                let _ = answer.send(());
                return false;
            }
            _ => {}
        }
        self.advance(epoch);
        match job {
            Job::Batch(batch) => {
                if !batch.is_empty() {
                    self.waiting.push_back(batch);
                }
                self.send_waiting(epoch);
            }
            Job::Retry => {
                self.send_waiting(epoch);
                self.store_outputs(epoch, None);
            }
            Job::Occurrence(occurrence) => {
                self.store_outputs(epoch, Some(Output::Occurrence(occurrence)));
            }
            Job::Episode(episode) => self.store_outputs(epoch, Some(Output::Episode(episode))),
            Job::Machine(machine) => {
                if let Err(err) = self.store.set_machine(&machine) {
                    self.report(&err);
                }
            }
            #[cfg(test)]
            Job::Run(run) => run(&mut self.store),
            Job::Flush(_) | Job::Stop(_) => {}
        }
        true
    }

    /// On the first job of a new epoch, drops what waited from the one before; only the newest
    /// event bookmark of it is kept, which a clear leaves in place.
    fn advance(&mut self, epoch: u64) {
        if epoch <= self.seen_epoch {
            return;
        }
        self.seen_epoch = epoch;
        let bookmark = self
            .waiting
            .iter()
            .rev()
            .find_map(|b| b.bookmark_xml.clone());
        self.waiting.clear();
        self.waiting.extend(bookmark.map(|xml| Batch {
            bookmark_xml: Some(xml),
            ..Batch::default()
        }));
        self.retrying
            .store(!self.waiting.is_empty(), Ordering::Release);
        self.unstored.clear();
        self.holding.store(false, Ordering::Release);
    }

    /// Stores a batch made in `epoch`; `false` when a clear came after it, in which case only its
    /// bookmark is stored.
    fn commit(&mut self, epoch: u64, batch: &Batch) -> Result<bool, StoreError> {
        let _held = self.links.epoch.hold();
        if epoch < self.links.epoch.current() {
            return match bookmark_only(batch) {
                Some(kept) => self.store.append_batch(&kept).map(|()| false),
                None => Ok(false),
            };
        }
        self.store.append_batch(batch).map(|()| true)
    }

    /// Writes the waiting batches oldest first and stops at the first failure; beyond the cap
    /// the oldest loses its samples.
    fn send_waiting(&mut self, epoch: u64) {
        while let Some(batch) = self.waiting.pop_front() {
            let err = match self.commit(epoch, &batch) {
                Ok(stored) => {
                    if stored {
                        self.ask_pass_after_events(&batch);
                    }
                    continue;
                }
                Err(err) => err,
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
        self.retrying
            .store(!self.waiting.is_empty(), Ordering::Release);
    }

    /// Stores `new` after the occurrences and episodes that failed before it; one that fails again
    /// waits for the next try, and past the cap the oldest is given up and the sampler told.
    fn store_outputs(&mut self, epoch: u64, new: Option<Output>) {
        let earlier = mem::take(&mut self.unstored);
        for output in earlier.into_iter().chain(new) {
            match self.commit_output(epoch, &output) {
                Ok(Written::Done(Some(latest))) => lock(&self.links.now).latest = Some(latest),
                Ok(Written::Done(None) | Written::Stale) => {}
                Err(err) => {
                    self.report(&err);
                    self.unstored.push_back(output);
                }
            }
        }
        while self.unstored.len() > OUTPUTS_KEPT
            && let Some(oldest) = self.unstored.pop_front()
        {
            let _ = self.links.dropped.send(oldest);
        }
        self.holding
            .store(!self.unstored.is_empty(), Ordering::Release);
    }

    fn commit_output(&mut self, epoch: u64, output: &Output) -> Result<Written, StoreError> {
        let _held = self.links.epoch.hold();
        if epoch < self.links.epoch.current() {
            return Ok(Written::Stale);
        }
        match output {
            Output::Occurrence(occurrence) => {
                let id = self.store.insert_occurrence(occurrence)?;
                Ok(Written::Done(Some(OccurrenceSummary {
                    id,
                    kind: occurrence.kind,
                    start_ms: occurrence.start_ms,
                    end_ms: occurrence.end_ms,
                    severity: occurrence.severity,
                    cause: occurrence.cause(),
                })))
            }
            Output::Episode(episode) => self
                .store
                .insert_episode(episode)
                .map(|_| Written::Done(None)),
            Output::Gap(_) => Ok(Written::Done(None)),
        }
    }

    /// The first stored batch that holds events asks for a health pass, which reads stored events.
    fn ask_pass_after_events(&mut self, batch: &Batch) {
        if self.event_pass_pending && !batch.events.is_empty() {
            self.event_pass_pending = false;
            self.links.health_due.store(true, Ordering::Release);
        }
    }

    fn shed_oldest(&mut self) {
        let Some(oldest) = self.waiting.pop_front() else {
            return;
        };
        if let Some(next) = self.waiting.front_mut() {
            shed_into(oldest, next);
        }
    }

    fn report(&self, err: &dyn std::fmt::Display) {
        lock(&self.links.status).last_error = Some(err.to_string());
    }
}

/// What came of writing an occurrence or an episode.
enum Written {
    /// A clear came after it was made, so it was not stored.
    Stale,
    /// Stored; an occurrence comes with the summary the live frame shows.
    Done(Option<OccurrenceSummary>),
}

/// Drops the samples of `oldest` and records a gap over their span in `next`, which takes over its
/// events, gaps and bookmark.
pub fn shed_into(oldest: Batch, next: &mut Batch) {
    let span = oldest
        .segments
        .iter()
        .map(|s| s.start_ms)
        .min()
        .zip(oldest.segments.iter().map(|s| s.end_ms).max());
    next.events.splice(0..0, oldest.events);
    next.gaps.splice(0..0, oldest.gaps);
    next.gaps.extend(span.map(|(start_ms, end_ms)| Gap {
        start_ms,
        end_ms,
        reason: GapReason::NotSaved,
    }));
    next.bookmark_xml = next.bookmark_xml.take().or(oldest.bookmark_xml);
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

/// The event bookmark of a batch alone, or `None` when it has none.
fn bookmark_only(batch: &Batch) -> Option<Batch> {
    batch.bookmark_xml.clone().map(|xml| Batch {
        bookmark_xml: Some(xml),
        ..Batch::default()
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use vestigium_core::{Gap, Sample};
    use vestigium_store::{Segment, Series, encode};

    use super::*;
    use crate::testing::{Refusal, WriterLock, disk_event};

    const PATIENCE: Duration = Duration::from_secs(15);

    fn links() -> Links {
        Links {
            status: Arc::new(Mutex::new(AgentStatus::default())),
            epoch: Arc::new(Epoch::default()),
            health_due: Arc::new(AtomicBool::new(false)),
            now: Arc::new(Mutex::new(NowFrame::default())),
            dropped: mpsc::channel().0,
        }
    }

    struct Rig {
        dir: tempfile::TempDir,
        service: Service,
        links: Links,
    }

    impl Rig {
        fn new() -> Rig {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
            let links = links();
            let service = Service::start(
                store,
                Links {
                    status: links.status.clone(),
                    epoch: links.epoch.clone(),
                    health_due: links.health_due.clone(),
                    now: links.now.clone(),
                    dropped: links.dropped.clone(),
                },
            )
            .unwrap();
            Rig {
                dir,
                service,
                links,
            }
        }

        fn history(&self) -> Store {
            Store::open(&self.dir.path().join("vestigium.db")).unwrap()
        }

        fn send(&self, job: Job) {
            let epoch = self.service.epoch();
            assert!(self.service.submit(Queued { epoch, job }).is_ok());
        }

        fn batch(&self, batch: Batch) {
            self.send(Job::Batch(batch));
        }

        fn last_error(&self) -> Option<String> {
            lock(&self.links.status).last_error.clone()
        }
    }

    fn segment(start_ms: i64, end_ms: i64) -> Segment {
        Segment {
            series: Series::Samples,
            start_ms,
            end_ms,
            data: encode::<Sample>(&[]),
        }
    }

    fn event_ids(store: &Store) -> Vec<u64> {
        store
            .events(i64::MIN, i64::MAX)
            .unwrap()
            .iter()
            .map(|e| e.record_id)
            .collect()
    }

    fn stored_segments(store: &Store) -> usize {
        store
            .segments(Series::Samples, i64::MIN, i64::MAX)
            .unwrap()
            .len()
    }

    #[test]
    fn submit_never_blocks_while_the_worker_is_busy() {
        let rig = Rig::new();
        rig.send(Job::Run(Box::new(|_| {
            thread::sleep(Duration::from_secs(2));
        })));

        let started = Instant::now();
        let mut refused = 0;
        for n in 0..5_000u64 {
            let queued = Queued {
                epoch: 0,
                job: Job::Batch(Batch {
                    events: vec![disk_event(n)],
                    ..Batch::default()
                }),
            };
            if rig.service.submit(queued).is_err() {
                refused += 1;
            }
        }

        assert!(
            started.elapsed() < Duration::from_millis(500),
            "5000 submits took {:?}",
            started.elapsed()
        );
        assert!(refused >= 5_000 - QUEUE_BOUND, "the excess comes back");
    }

    #[test]
    fn jobs_run_in_order() {
        let rig = Rig::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        for n in 0..10 {
            let seen = seen.clone();
            rig.send(Job::Run(Box::new(move |_| lock(&seen).push(n))));
        }

        assert!(rig.service.flush(PATIENCE));

        assert_eq!(*lock(&seen), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn stop_drains_the_queue() {
        let rig = Rig::new();
        for n in 1..=5u64 {
            rig.batch(Batch {
                events: vec![disk_event(n)],
                bookmark_xml: Some(format!("bookmark {n}")),
                ..Batch::default()
            });
        }

        assert!(rig.service.stop(PATIENCE));

        let store = rig.history();
        assert_eq!(event_ids(&store), [1, 2, 3, 4, 5]);
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("bookmark 5"));
        let late = Queued {
            epoch: 0,
            job: Job::Batch(Batch::default()),
        };
        assert!(
            rig.service.submit(late).is_err(),
            "nothing is taken once the thread ended"
        );
    }

    #[test]
    fn a_job_that_panics_does_not_end_the_thread() {
        let rig = Rig::new();
        rig.send(Job::Run(Box::new(|_| panic!("a bad job"))));
        rig.batch(Batch {
            events: vec![disk_event(1)],
            ..Batch::default()
        });

        assert!(rig.service.flush(PATIENCE));

        assert!(rig.last_error().unwrap().contains("a bad job"));
        assert_eq!(event_ids(&rig.history()), [1]);
    }

    #[test]
    fn a_batch_that_fails_to_write_is_stored_with_the_next_one() {
        let rig = Rig::new();
        let held = WriterLock::take(rig.dir.path());
        rig.batch(Batch {
            segments: vec![segment(1_000, 2_000)],
            events: vec![disk_event(1)],
            bookmark_xml: Some("first".to_string()),
            ..Batch::default()
        });
        assert!(rig.service.flush(PATIENCE));
        drop(held);
        assert!(rig.last_error().is_some());
        assert!(rig.service.is_retrying());

        rig.batch(Batch {
            events: vec![disk_event(2)],
            bookmark_xml: Some("second".to_string()),
            ..Batch::default()
        });
        assert!(rig.service.flush(PATIENCE));

        let store = rig.history();
        assert_eq!(event_ids(&store), [1, 2]);
        assert_eq!(stored_segments(&store), 1);
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("second"));
        assert!(!rig.service.is_retrying());
    }

    #[test]
    fn an_empty_batch_tries_the_waiting_ones_again() {
        let rig = Rig::new();
        let refusal = Refusal::of(rig.dir.path(), "event");
        rig.batch(Batch {
            events: vec![disk_event(1)],
            ..Batch::default()
        });
        assert!(rig.service.flush(PATIENCE));
        assert!(rig.service.is_retrying());
        drop(refusal);

        rig.send(Job::Retry);
        assert!(rig.service.flush(PATIENCE));

        assert_eq!(event_ids(&rig.history()), [1]);
    }

    #[test]
    fn batches_that_keep_failing_wait_up_to_the_cap_then_lose_their_samples() {
        let rig = Rig::new();
        let refusal = Refusal::of(rig.dir.path(), "segment");
        for n in 1..=4u64 {
            rig.batch(Batch {
                segments: vec![segment(n as i64 * 60_000, n as i64 * 60_000 + 1_000)],
                events: vec![disk_event(n)],
                bookmark_xml: Some(format!("bookmark {n}")),
                ..Batch::default()
            });
        }
        assert!(rig.service.flush(PATIENCE));
        drop(refusal);

        rig.send(Job::Retry);
        assert!(rig.service.flush(PATIENCE));

        let store = rig.history();
        assert_eq!(event_ids(&store), [1, 2, 3, 4]);
        assert_eq!(stored_segments(&store), BATCHES_KEPT);
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("bookmark 4"));
        assert_eq!(
            store.gaps(i64::MIN, i64::MAX).unwrap(),
            vec![Gap {
                start_ms: 60_000,
                end_ms: 61_000,
                reason: GapReason::NotSaved,
            }]
        );
    }

    #[test]
    fn a_batch_without_samples_that_is_dropped_leaves_no_gap() {
        let rig = Rig::new();
        let refusal = Refusal::of(rig.dir.path(), "event");
        for n in 1..=4u64 {
            rig.batch(Batch {
                events: vec![disk_event(n)],
                ..Batch::default()
            });
        }
        assert!(rig.service.flush(PATIENCE));
        drop(refusal);

        rig.send(Job::Retry);
        assert!(rig.service.flush(PATIENCE));

        assert!(rig.history().gaps(i64::MIN, i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn a_batch_with_an_unstorable_value_keeps_only_samples_and_bookmark() {
        let batch = Batch {
            segments: vec![segment(1, 2)],
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

    #[test]
    fn the_first_stored_batch_with_events_asks_for_a_health_pass() {
        let rig = Rig::new();
        rig.batch(Batch {
            segments: vec![segment(1, 2)],
            ..Batch::default()
        });
        assert!(rig.service.flush(PATIENCE));
        assert!(
            !rig.links.health_due.load(Ordering::Acquire),
            "a batch without events asks nothing"
        );

        rig.batch(Batch {
            events: vec![disk_event(1)],
            ..Batch::default()
        });
        assert!(rig.service.flush(PATIENCE));
        assert!(rig.links.health_due.load(Ordering::Acquire));

        rig.links.health_due.store(false, Ordering::Release);
        rig.batch(Batch {
            events: vec![disk_event(2)],
            ..Batch::default()
        });
        assert!(rig.service.flush(PATIENCE));
        assert!(
            !rig.links.health_due.load(Ordering::Acquire),
            "later batches do not ask again"
        );
    }

    #[test]
    fn a_batch_queued_before_a_clear_is_not_written() {
        let rig = Rig::new();
        let release = Arc::new(AtomicBool::new(false));
        let waited = release.clone();
        rig.send(Job::Run(Box::new(move |_| {
            while !waited.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
            }
        })));
        rig.batch(Batch {
            segments: vec![segment(1_000, 2_000)],
            events: vec![disk_event(1)],
            bookmark_xml: Some("kept".to_string()),
            gaps: vec![Gap {
                start_ms: 1,
                end_ms: 2,
                reason: GapReason::Paused,
            }],
        });

        let mut pipe = rig.history();
        rig.links.epoch.clear(|| pipe.clear_history()).unwrap();
        release.store(true, Ordering::Release);
        assert!(rig.service.flush(PATIENCE));

        let store = rig.history();
        assert!(event_ids(&store).is_empty());
        assert!(store.gaps(i64::MIN, i64::MAX).unwrap().is_empty());
        assert_eq!(stored_segments(&store), 0);
        assert_eq!(
            store.bookmark_xml().unwrap().as_deref(),
            Some("kept"),
            "a clear keeps the event bookmark, so the one queued before it is kept too"
        );
    }

    #[test]
    fn a_batch_made_after_a_clear_is_written() {
        let rig = Rig::new();
        let mut pipe = rig.history();
        rig.links.epoch.clear(|| pipe.clear_history()).unwrap();

        rig.send(Job::Batch(Batch {
            segments: vec![segment(1_000, 2_000)],
            events: vec![disk_event(7)],
            ..Batch::default()
        }));
        assert!(rig.service.flush(PATIENCE));

        let store = rig.history();
        assert_eq!(event_ids(&store), [7]);
        assert_eq!(stored_segments(&store), 1);
    }

    #[test]
    fn a_failed_clear_starts_no_epoch() {
        let epoch = Epoch::default();

        let refused: Result<(), &str> = epoch.clear(|| Err("refused"));

        assert!(refused.is_err());
        assert_eq!(epoch.current(), 0);
    }

    #[test]
    fn a_clear_waits_for_the_write_in_progress() {
        let rig = Rig::new();
        let writing = Arc::new(AtomicUsize::new(0));
        let during = writing.clone();
        let epoch = rig.links.epoch.clone();
        let clearing = thread::spawn(move || {
            while during.load(Ordering::Acquire) == 0 {
                thread::sleep(Duration::from_millis(1));
            }
            let started = Instant::now();
            epoch.clear(|| Ok::<_, ()>(())).unwrap();
            started.elapsed()
        });
        let held = rig.links.epoch.clone();
        let hold = Arc::new(AtomicBool::new(false));
        let release = hold.clone();
        rig.send(Job::Run(Box::new(move |_| {
            let _in_a_write = held.hold();
            writing.store(1, Ordering::Release);
            while !release.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
            }
        })));
        thread::sleep(Duration::from_millis(100));
        hold.store(true, Ordering::Release);

        let waited = clearing.join().unwrap();

        assert!(waited >= Duration::from_millis(50), "waited {waited:?}");
        assert_eq!(rig.links.epoch.current(), 1);
    }
}
