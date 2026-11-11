use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;

use vestigium_collect::{HealthSource, physical_drive};
use vestigium_core::health::{
    Concern, DiskHealth, FindingChange, FindingKind, HealthInputs, PageFile, SecurityProduct,
    Subject, VolumeSpace, evaluate, reconcile, unread_kinds,
};
use vestigium_core::hwtest::{Bus, TEST_REACH_DAYS};
use vestigium_core::{EpisodeKind, MachineInfo, Media};
use vestigium_ipc::AgentStatus;
use vestigium_store::{Store, StoreError};

use crate::app::{lock, panic_text};
use crate::sampler::release_idle_pages;
use crate::service::{Job, Queued, Service};

const DAY_MS: i64 = 24 * 60 * 60_000;
/// Event history one pass reads: the 30-day counts and the four weeks a trend compares against.
const EVENT_DAYS: i64 = 35;
/// How often the Security Center is asked; its COM libraries stay loaded after a call.
const ANTIVIRUS_EVERY_MS: i64 = DAY_MS;
/// Stack of the reader thread: it calls into Windows and holds no deep call chain.
const STACK_BYTES: usize = 512 << 10;

/// What the sampler knows at the moment of a pass, owned so it can go to the reader thread.
#[derive(Clone, Debug)]
pub struct PassRequest {
    pub ts_ms: i64,
    pub machine: MachineInfo,
    /// Disks with I/O in the last minute; an idle HDD is not asked, so it never spins up for us.
    pub busy_disks: HashSet<String>,
    pub commit_limit_bytes: u64,
}

/// What the sources said in one pass, to be evaluated and stored on the storage thread.
#[derive(Clone, Debug)]
pub struct HealthReadings {
    ts_ms: i64,
    machine: MachineInfo,
    commit_limit_bytes: u64,
    /// The disks read in this pass.
    fresh: Vec<DiskHealth>,
    /// The latest reading of every disk: this pass's, and the kept one of a disk left alone.
    disks: Vec<DiskHealth>,
    /// Disks this pass did not read.
    unread: HashSet<String>,
    /// Disks this pass asked and got no answer from; an idle HDD left alone is not one.
    failed: HashSet<String>,
    system_disk: Option<String>,
    volumes: Vec<VolumeSpace>,
    page_files: Option<Vec<PageFile>>,
    antivirus: Option<Vec<SecurityProduct>>,
    /// The Security Center was asked, so the pages it touched can be handed back.
    asked_security_center: bool,
}

impl HealthReadings {
    /// When the pass was asked for; Unix epoch, milliseconds.
    pub fn ts_ms(&self) -> i64 {
        self.ts_ms
    }
}

/// What one pass did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PassOutcome {
    pub changes: usize,
    /// Findings rated critical after this pass.
    pub critical: u32,
    /// Finding kinds a source gave no answer for, so the pass could not tell if they hold.
    pub unread: Vec<FindingKind>,
}

/// Reads the health sources. Any of them may block for seconds, so this runs on its own thread.
pub struct HealthReader<S> {
    source: S,
    /// Last reading of each disk, kept while an HDD sleeps.
    disks: BTreeMap<String, DiskHealth>,
    antivirus: Option<Vec<SecurityProduct>>,
    antivirus_read_ms: Option<i64>,
}

impl<S: HealthSource> HealthReader<S> {
    pub fn new(source: S) -> HealthReader<S> {
        HealthReader {
            source,
            disks: BTreeMap::new(),
            antivirus: None,
            antivirus_read_ms: None,
        }
    }

    /// An idle disk is left alone when it is an HDD, or when its media is unknown and it is not NVMe.
    fn skips_idle(&mut self, index: u32, media: Media, busy: bool) -> bool {
        !busy
            && match media {
                Media::Hdd => true,
                Media::Unknown => self.source.bus(index) != Bus::Nvme,
                _ => false,
            }
    }

    /// Asks every source what it knows; nothing here touches the history.
    pub fn read(&mut self, request: &PassRequest) -> HealthReadings {
        // A failed read is asked again on the next pass; only an answer is kept for a day.
        let asked_security_center = self.antivirus.is_none()
            || self
                .antivirus_read_ms
                .is_none_or(|at| request.ts_ms - at >= ANTIVIRUS_EVERY_MS);
        if asked_security_center {
            self.antivirus = self.source.antivirus();
            self.antivirus_read_ms = self.antivirus.is_some().then_some(request.ts_ms);
        }
        let mut unread: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();
        let mut fresh = Vec::new();
        for disk in &request.machine.disks {
            let Some(index) = physical_drive(&disk.name) else {
                continue;
            };
            if self.skips_idle(index, disk.media, request.busy_disks.contains(&disk.name)) {
                unread.insert(disk.name.clone());
                continue;
            }
            let Some(health) = self.source.disk(index, &disk.name, request.ts_ms) else {
                unread.insert(disk.name.clone());
                failed.insert(disk.name.clone());
                continue;
            };
            fresh.push(health.clone());
            self.disks.insert(disk.name.clone(), health);
        }
        let in_machine = |name: &String| request.machine.disks.iter().any(|d| &d.name == name);
        self.disks.retain(|name, _| in_machine(name));
        let system_disk = self.source.system_drive().and_then(|number| {
            request
                .machine
                .disks
                .iter()
                .find(|d| physical_drive(&d.name) == Some(number))
                .map(|d| d.name.clone())
        });
        HealthReadings {
            ts_ms: request.ts_ms,
            machine: request.machine.clone(),
            commit_limit_bytes: request.commit_limit_bytes,
            fresh,
            disks: self.disks.values().cloned().collect(),
            unread,
            failed,
            system_disk,
            volumes: self.source.volumes(),
            page_files: self.source.page_files(),
            antivirus: self.antivirus.clone(),
            asked_security_center,
        }
    }
}

/// Evaluates the readings of a pass against the history and stores what changed.
#[derive(Default)]
pub struct HealthKeeper {
    /// The UTC day of each disk's last stored SMART reading.
    smart_day: BTreeMap<String, i64>,
}

impl HealthKeeper {
    /// Forgets the days of the stored SMART readings, which a cleared history no longer holds.
    pub fn forget(&mut self) {
        self.smart_day.clear();
    }

    pub fn store(
        &mut self,
        store: &mut Store,
        readings: &HealthReadings,
    ) -> Result<PassOutcome, StoreError> {
        for health in &readings.fresh {
            let day = health.read_ms.div_euclid(DAY_MS);
            if !health.attributes.is_empty() && self.smart_day.get(&health.name) != Some(&day) {
                store.record_smart(health)?;
                self.smart_day.insert(health.name.clone(), day);
            }
        }
        let machine = &readings.machine;
        let now_ms = readings.ts_ms;
        self.smart_day
            .retain(|name, _| machine.disks.iter().any(|d| &d.name == name));

        let week_ago = now_ms - 7 * DAY_MS;
        let open = store.open_findings()?;
        let commit_peak_pct_7d = (readings.commit_limit_bytes > 0)
            .then(|| store.minutes(week_ago, now_ms))
            .transpose()?
            .and_then(|minutes| minutes.iter().map(|m| m.commit_max).max())
            .map(|peak| peak as f32 * 100.0 / readings.commit_limit_bytes as f32);
        let inputs = HealthInputs {
            now_ms,
            history_from_ms: store.first_segment_ms()?.unwrap_or(now_ms),
            machine: machine.clone(),
            system_disk: readings.system_disk.clone(),
            disks: readings.disks.clone(),
            volumes: readings.volumes.clone(),
            page_files: readings.page_files.clone(),
            commit_peak_pct_7d,
            memory_high_days_7d: memory_high_days(store, week_ago, now_ms)?,
            events: store.events(now_ms - EVENT_DAYS * DAY_MS, now_ms)?,
            antivirus: readings.antivirus.clone(),
            boot: store.last_scan()?.and_then(|scan| scan.boot),
            tests: store
                .test_runs(now_ms - TEST_REACH_DAYS * DAY_MS, now_ms)?
                .into_iter()
                .map(|row| row.run)
                .collect(),
            open: open
                .iter()
                .map(|s| (s.finding.kind, s.finding.subject.clone()))
                .collect(),
        };
        let fresh = evaluate(&inputs);
        let mut unread = unread_kinds(&inputs);
        let mut changes = reconcile(&open, &fresh, &unread);
        // A disk this pass did not read keeps its open SMART finding.
        changes.retain(|change| match change {
            FindingChange::Close { id } => !open.iter().any(|s| {
                s.id == *id
                    && s.finding.kind == FindingKind::DiskSmart
                    && matches!(&s.finding.subject, Subject::Disk(name) if readings.unread.contains(name.as_str()))
            }),
            _ => true,
        });
        store.apply_findings(&changes, now_ms)?;
        if !readings.failed.is_empty() {
            unread.push(FindingKind::DiskSmart);
        }
        Ok(PassOutcome {
            changes: changes.len(),
            unread,
            critical: fresh
                .iter()
                .filter(|f| f.concern == Concern::Critical)
                .count() as u32,
        })
    }
}

/// A health source of any kind, so the sampler can be wired to this PC or to a stand-in for it.
pub struct AnySource(pub Box<dyn HealthSource + Send>);

impl HealthSource for AnySource {
    fn disk(&mut self, index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
        self.0.disk(index, name, now_ms)
    }

    fn bus(&mut self, index: u32) -> Bus {
        self.0.bus(index)
    }

    fn volumes(&mut self) -> Vec<VolumeSpace> {
        self.0.volumes()
    }

    fn system_drive(&mut self) -> Option<u32> {
        self.0.system_drive()
    }

    fn page_files(&mut self) -> Option<Vec<PageFile>> {
        self.0.page_files()
    }

    fn antivirus(&mut self) -> Option<Vec<SecurityProduct>> {
        self.0.antivirus()
    }
}

/// The thread that reads the health sources and hands the readings to the storage thread. It
/// holds at most one request: while it is stuck in a call, a newer one replaces the one waiting.
pub struct HealthThread {
    slot: Arc<Slot>,
}

struct Slot {
    state: Mutex<SlotState>,
    wake: Condvar,
}

#[derive(Default)]
struct SlotState {
    /// The epoch the request was made in, and the request.
    pending: Option<(u64, PassRequest)>,
    closed: bool,
}

impl HealthThread {
    /// Starts the thread; it ends once this handle is dropped and its current call returns.
    pub fn start<S>(
        mut reader: HealthReader<S>,
        service: Service,
        status: Arc<Mutex<AgentStatus>>,
    ) -> io::Result<HealthThread>
    where
        S: HealthSource + Send + 'static,
    {
        let slot = Arc::new(Slot {
            state: Mutex::new(SlotState::default()),
            wake: Condvar::new(),
        });
        let mine = slot.clone();
        thread::Builder::new()
            .name("health".to_string())
            .stack_size(STACK_BYTES)
            .spawn(move || {
                while let Some((epoch, request)) = mine.next() {
                    let read = panic::catch_unwind(AssertUnwindSafe(|| reader.read(&request)));
                    match read {
                        Ok(readings) => {
                            if readings.asked_security_center {
                                release_idle_pages();
                            }
                            let job = Job::Health(Box::new(readings));
                            if !service.deliver(Queued { epoch, job }) {
                                return;
                            }
                        }
                        Err(payload) => {
                            let mut status = lock(&status);
                            status.monitor.health_attempt_ms = Some(request.ts_ms);
                            status.last_error = Some(format!(
                                "a health read stopped: {}",
                                panic_text(payload.as_ref())
                            ));
                        }
                    }
                }
            })?;
        Ok(HealthThread { slot })
    }

    /// Asks for a pass; a request still waiting is replaced, because the newer one is fresher.
    pub fn request(&self, epoch: u64, request: PassRequest) {
        lock(&self.slot.state).pending = Some((epoch, request));
        self.slot.wake.notify_one();
    }
}

impl Drop for HealthThread {
    fn drop(&mut self) {
        lock(&self.slot.state).closed = true;
        self.slot.wake.notify_one();
    }
}

impl Slot {
    /// Blocks until a request comes, or `None` once the handle is gone.
    fn next(&self) -> Option<(u64, PassRequest)> {
        let mut state = lock(&self.state);
        loop {
            if state.closed {
                return None;
            }
            if let Some(request) = state.pending.take() {
                return Some(request);
            }
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Days of `[from_ms, to_ms]`, counted back from `to_ms`, that saw a high-memory episode.
fn memory_high_days(store: &Store, from_ms: i64, to_ms: i64) -> Result<u32, StoreError> {
    let days: BTreeSet<i64> = store
        .episodes(from_ms, to_ms)?
        .iter()
        .filter(|e| e.kind == EpisodeKind::MemoryHigh)
        .flat_map(|e| {
            let first = (to_ms - e.end_ms.min(to_ms)).div_euclid(DAY_MS);
            let last = (to_ms - e.start_ms.max(from_ms)).div_euclid(DAY_MS);
            first..=last
        })
        .collect();
    Ok(days.len() as u32)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use vestigium_core::{Channel, DiskInfo, Episode, EventMeaning, WinEvent};
    use vestigium_store::Batch;

    use super::*;
    use crate::service::Links;
    use crate::testing::wait_until;
    use vestigium_core::health::SmartAttribute;

    const GB: u64 = 1 << 30;
    const NOW: i64 = 100 * DAY_MS;

    /// A machine whose answers the test sets, counting what the pass asked.
    #[derive(Clone, Default)]
    struct FakeHealth {
        free_gb: u64,
        asked_disks: Rc<RefCell<Vec<String>>>,
        asked_antivirus: Rc<RefCell<u32>>,
        failing_disk: bool,
        bus: Option<Bus>,
        two_antivirus: bool,
        antivirus_down: Rc<Cell<bool>>,
        page_files_down: Rc<Cell<bool>>,
        disks_down: Rc<Cell<bool>>,
    }

    impl HealthSource for FakeHealth {
        fn disk(&mut self, _index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
            self.asked_disks.borrow_mut().push(name.to_string());
            if self.disks_down.get() {
                return None;
            }
            Some(DiskHealth {
                name: name.to_string(),
                read_ms: now_ms,
                predict_failure: self.failing_disk,
                attributes: vec![SmartAttribute {
                    id: 9,
                    current: 100,
                    worst: 100,
                    raw: now_ms.div_euclid(DAY_MS) as u64,
                }],
                nvme: None,
            })
        }

        fn bus(&mut self, _index: u32) -> Bus {
            self.bus.unwrap_or(Bus::Other)
        }

        fn volumes(&mut self) -> Vec<VolumeSpace> {
            vec![VolumeSpace {
                root: r"C:\".into(),
                free_bytes: self.free_gb * GB,
                total_bytes: 100 * GB,
                system: true,
            }]
        }

        fn system_drive(&mut self) -> Option<u32> {
            Some(0)
        }

        fn page_files(&mut self) -> Option<Vec<PageFile>> {
            if self.page_files_down.get() {
                return None;
            }
            Some(vec![PageFile {
                path: r"C:\pagefile.sys".into(),
                size_bytes: 8 * GB,
                peak_bytes: GB,
            }])
        }

        fn antivirus(&mut self) -> Option<Vec<SecurityProduct>> {
            *self.asked_antivirus.borrow_mut() += 1;
            if self.antivirus_down.get() {
                return None;
            }
            let mut products = vec![SecurityProduct {
                name: "Built-in antivirus".into(),
                on: true,
            }];
            if self.two_antivirus {
                products.push(SecurityProduct {
                    name: "Other antivirus".into(),
                    on: true,
                });
            }
            Some(products)
        }
    }

    fn machine() -> MachineInfo {
        MachineInfo {
            logical_cpus: 8,
            mem_total_bytes: 32 * GB,
            disks: vec![
                DiskInfo {
                    name: "0 C:".into(),
                    media: Media::Ssd,
                },
                DiskInfo {
                    name: "1 D:".into(),
                    media: Media::Hdd,
                },
            ],
        }
    }

    fn store(dir: &tempfile::TempDir) -> Store {
        Store::open(&dir.path().join("vestigium.db")).unwrap()
    }

    /// A reader and a keeper run one after the other, as the two threads do.
    struct HealthPass<S> {
        reader: HealthReader<S>,
        keeper: HealthKeeper,
    }

    /// What one pass did, with what its readings said about the Security Center.
    struct Ran {
        changes: usize,
        critical: u32,
        unread: Vec<FindingKind>,
        asked_security_center: bool,
    }

    impl<S: HealthSource> HealthPass<S> {
        fn new(source: S) -> HealthPass<S> {
            HealthPass {
                reader: HealthReader::new(source),
                keeper: HealthKeeper::default(),
            }
        }

        fn forget(&mut self) {
            self.keeper.forget();
        }

        fn run(&mut self, store: &mut Store, request: &PassRequest) -> Result<Ran, StoreError> {
            let readings = self.reader.read(request);
            let outcome = self.keeper.store(store, &readings)?;
            Ok(Ran {
                changes: outcome.changes,
                critical: outcome.critical,
                unread: outcome.unread,
                asked_security_center: readings.asked_security_center,
            })
        }
    }

    fn request(machine: &MachineInfo, at: i64, busy: &[&str]) -> PassRequest {
        PassRequest {
            ts_ms: at,
            machine: machine.clone(),
            busy_disks: busy.iter().map(|s| s.to_string()).collect(),
            commit_limit_bytes: 40 * GB,
        }
    }

    fn run(pass: &mut HealthPass<FakeHealth>, store: &mut Store, at: i64, busy: &[&str]) -> Ran {
        run_on(pass, store, &machine(), at, busy)
    }

    fn run_on(
        pass: &mut HealthPass<FakeHealth>,
        store: &mut Store,
        machine: &MachineInfo,
        at: i64,
        busy: &[&str],
    ) -> Ran {
        pass.run(store, &request(machine, at, busy)).unwrap()
    }

    #[test]
    fn a_pass_opens_and_later_closes_a_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 4,
            ..FakeHealth::default()
        };
        let mut pass = HealthPass::new(fake);

        let first = run(&mut pass, &mut store, NOW, &[]);
        assert_eq!(first.changes, 1);
        assert_eq!(first.critical, 1);
        assert_eq!(
            store.open_findings().unwrap()[0].finding.kind,
            FindingKind::LowSpace
        );

        pass.reader.source.free_gb = 50;
        let second = run(&mut pass, &mut store, NOW + DAY_MS / 24, &[]);
        assert_eq!(second.changes, 1);
        assert_eq!(second.critical, 0);
        assert!(store.open_findings().unwrap().is_empty());
    }

    #[test]
    fn smart_is_stored_once_a_day_per_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let mut pass = HealthPass::new(FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        });

        run(&mut pass, &mut store, NOW, &[]);
        run(&mut pass, &mut store, NOW + DAY_MS / 24, &[]);
        run(&mut pass, &mut store, NOW + DAY_MS, &[]);

        let days: Vec<i64> = store
            .smart_history("0 C:", 9, 0)
            .unwrap()
            .iter()
            .map(|r| r.read_ms)
            .collect();
        assert_eq!(days, [NOW, NOW + DAY_MS], "the first reading of each day");
        assert!(
            store.smart_history("1 D:", 9, 0).unwrap().is_empty(),
            "an idle HDD is never read"
        );
    }

    #[test]
    fn smart_is_stored_again_after_the_history_is_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let mut pass = HealthPass::new(FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        });
        run(&mut pass, &mut store, NOW, &[]);
        store.clear_history().unwrap();

        pass.forget();
        run(&mut pass, &mut store, NOW + DAY_MS / 24, &[]);

        assert_eq!(store.smart_history("0 C:", 9, 0).unwrap().len(), 1);
    }

    #[test]
    fn an_idle_hdd_is_never_asked() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            failing_disk: true,
            ..FakeHealth::default()
        };
        let asked = fake.asked_disks.clone();
        let mut pass = HealthPass::new(fake);

        run(&mut pass, &mut store, NOW, &[]);
        assert_eq!(*asked.borrow(), ["0 C:"]);

        run(&mut pass, &mut store, NOW + 60_000, &["1 D:"]);
        assert_eq!(*asked.borrow(), ["0 C:", "0 C:", "1 D:"]);
        let failing: Vec<_> = store
            .open_findings()
            .unwrap()
            .into_iter()
            .filter(|f| f.finding.kind == FindingKind::DiskSmart)
            .collect();
        assert_eq!(
            failing.len(),
            2,
            "the HDD's last reading stays once it sleeps again"
        );
    }

    #[test]
    fn an_idle_disk_of_unknown_media_is_not_asked_unless_nvme() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let mut machine = machine();
        machine.disks[1].media = Media::Unknown;
        let other = FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        };
        let asked = other.asked_disks.clone();
        let mut pass = HealthPass::new(other);

        run_on(&mut pass, &mut store, &machine, NOW, &[]);
        assert_eq!(*asked.borrow(), ["0 C:"]);
        run_on(&mut pass, &mut store, &machine, NOW + 60_000, &["1 D:"]);
        assert_eq!(
            *asked.borrow(),
            ["0 C:", "0 C:", "1 D:"],
            "a busy disk is read"
        );

        let nvme = FakeHealth {
            free_gb: 50,
            bus: Some(Bus::Nvme),
            ..FakeHealth::default()
        };
        let asked = nvme.asked_disks.clone();
        let mut pass = HealthPass::new(nvme);
        run_on(&mut pass, &mut store, &machine, NOW, &[]);
        assert_eq!(*asked.borrow(), ["0 C:", "1 D:"]);
    }

    #[test]
    fn a_disk_that_left_the_machine_stops_being_reported() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let mut pass = HealthPass::new(FakeHealth {
            free_gb: 50,
            failing_disk: true,
            ..FakeHealth::default()
        });
        let smart_of = |store: &Store, name: &str| {
            store
                .open_findings()
                .unwrap()
                .into_iter()
                .filter(|f| f.finding.kind == FindingKind::DiskSmart)
                .filter(|f| matches!(&f.finding.subject, Subject::Disk(disk) if disk == name))
                .count()
        };

        run(&mut pass, &mut store, NOW, &["1 D:"]);
        assert_eq!(smart_of(&store, "1 D:"), 1);

        let mut without_d = machine();
        without_d.disks.truncate(1);
        run_on(&mut pass, &mut store, &without_d, NOW + 60_000, &[]);

        assert_eq!(smart_of(&store, "1 D:"), 0);
        assert_eq!(smart_of(&store, "0 C:"), 1);
    }

    #[test]
    fn a_fresh_pass_keeps_the_smart_finding_of_an_idle_hdd() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            failing_disk: true,
            ..FakeHealth::default()
        };
        let mut first = HealthPass::new(fake.clone());
        run(&mut first, &mut store, NOW, &["1 D:"]);
        let smart_of_hdd = |store: &Store| {
            store.open_findings().unwrap().into_iter().find(|f| {
                f.finding.kind == FindingKind::DiskSmart
                    && f.finding.subject == Subject::Disk("1 D:".into())
            })
        };
        let opened = smart_of_hdd(&store)
            .expect("the HDD's finding opens")
            .opened_ms;

        let mut restarted = HealthPass::new(fake);
        run(&mut restarted, &mut store, NOW + 60_000, &[]);

        let kept = smart_of_hdd(&store).expect("the idle HDD's finding stays open");
        assert_eq!(kept.opened_ms, opened);
    }

    #[test]
    fn the_security_center_is_asked_once_a_day() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        };
        let asked = fake.asked_antivirus.clone();
        let mut pass = HealthPass::new(fake);

        assert!(run(&mut pass, &mut store, NOW, &[]).asked_security_center);
        assert!(!run(&mut pass, &mut store, NOW + 3_600_000, &[]).asked_security_center);
        assert!(run(&mut pass, &mut store, NOW + DAY_MS, &[]).asked_security_center);
        assert_eq!(*asked.borrow(), 2);
    }

    fn open_kinds(store: &Store) -> Vec<FindingKind> {
        store
            .open_findings()
            .unwrap()
            .iter()
            .map(|f| f.finding.kind)
            .collect()
    }

    #[test]
    fn an_antivirus_query_that_fails_is_reported_unread() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        };
        let down = fake.antivirus_down.clone();
        let mut pass = HealthPass::new(fake);

        down.set(true);
        let failed = run(&mut pass, &mut store, NOW, &[]);
        down.set(false);
        let answered = run(&mut pass, &mut store, NOW + 60 * 60_000, &[]);

        assert_eq!(failed.unread, [FindingKind::TwoAntivirus]);
        assert!(answered.unread.is_empty());
    }

    #[test]
    fn a_disk_that_answers_nothing_is_unread_but_an_idle_hdd_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        };
        let down = fake.disks_down.clone();
        let mut pass = HealthPass::new(fake);

        let idle = run(&mut pass, &mut store, NOW, &[]);
        down.set(true);
        let silent = run(&mut pass, &mut store, NOW + 60 * 60_000, &[]);
        let busy = run(&mut pass, &mut store, NOW + 120 * 60_000, &["1 D:"]);

        assert!(
            !idle.unread.contains(&FindingKind::DiskSmart),
            "an idle HDD is left alone"
        );
        assert!(
            silent.unread.contains(&FindingKind::DiskSmart),
            "the SSD was asked"
        );
        assert!(busy.unread.contains(&FindingKind::DiskSmart));
    }

    #[test]
    fn a_failed_antivirus_read_keeps_the_open_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            two_antivirus: true,
            ..FakeHealth::default()
        };
        let down = fake.antivirus_down.clone();
        let asked = fake.asked_antivirus.clone();
        let mut pass = HealthPass::new(fake);
        run(&mut pass, &mut store, NOW, &[]);
        assert_eq!(open_kinds(&store), [FindingKind::TwoAntivirus]);

        down.set(true);
        run(&mut pass, &mut store, NOW + DAY_MS, &[]);
        assert_eq!(open_kinds(&store), [FindingKind::TwoAntivirus]);

        run(&mut pass, &mut store, NOW + DAY_MS + 3_600_000, &[]);
        assert_eq!(*asked.borrow(), 3, "a failed read is asked again next pass");
        assert_eq!(open_kinds(&store), [FindingKind::TwoAntivirus]);

        down.set(false);
        run(&mut pass, &mut store, NOW + DAY_MS + 7_200_000, &[]);
        assert_eq!(open_kinds(&store), [FindingKind::TwoAntivirus]);
    }

    #[test]
    fn a_fresh_pass_without_the_security_center_keeps_two_antivirus() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            two_antivirus: true,
            ..FakeHealth::default()
        };
        let mut first = HealthPass::new(fake.clone());
        run(&mut first, &mut store, NOW, &[]);

        let down = fake.clone();
        down.antivirus_down.set(true);
        let mut restarted = HealthPass::new(down);
        run(&mut restarted, &mut store, NOW + 60_000, &[]);

        assert_eq!(open_kinds(&store), [FindingKind::TwoAntivirus]);
    }

    #[test]
    fn an_antivirus_that_comes_up_later_opens_the_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            two_antivirus: true,
            ..FakeHealth::default()
        };
        fake.antivirus_down.set(true);
        let down = fake.antivirus_down.clone();
        let mut pass = HealthPass::new(fake);
        run(&mut pass, &mut store, NOW, &[]);
        assert!(open_kinds(&store).is_empty());

        down.set(false);
        run(&mut pass, &mut store, NOW + 3_600_000, &[]);
        assert_eq!(open_kinds(&store), [FindingKind::TwoAntivirus]);
    }

    #[test]
    fn a_page_file_read_that_fails_keeps_the_open_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let fake = FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        };
        let down = fake.page_files_down.clone();
        let mut pass = HealthPass::new(fake);
        let event = WinEvent {
            ts_ms: NOW - DAY_MS,
            channel: Channel::System,
            provider: "Microsoft-Windows-Resource-Exhaustion-Detector".into(),
            id: 2004,
            record_id: 1,
            level: 2,
            meaning: EventMeaning::LowVirtualMemory,
            data_xml: String::new(),
        };
        store
            .append_batch(&Batch {
                events: vec![event],
                ..Batch::default()
            })
            .unwrap();
        run(&mut pass, &mut store, NOW, &[]);
        assert_eq!(open_kinds(&store), [FindingKind::PageFile]);

        down.set(true);
        run(&mut pass, &mut store, NOW + 3_600_000, &[]);
        assert_eq!(open_kinds(&store), [FindingKind::PageFile]);
    }

    #[test]
    fn memory_days_count_each_day_an_episode_touched() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let episode = |start_ms, end_ms| Episode {
            kind: EpisodeKind::MemoryHigh,
            start_ms,
            end_ms,
            peak: 91.0,
            culprit: None,
            truncated: false,
        };
        store
            .insert_episode(&episode(NOW - 3_600_000, NOW - 1_800_000))
            .unwrap();
        store
            .insert_episode(&episode(NOW - DAY_MS - 600_000, NOW - DAY_MS + 600_000))
            .unwrap();
        store
            .insert_episode(&episode(NOW - 9 * DAY_MS, NOW - 8 * DAY_MS))
            .unwrap();
        store
            .insert_episode(&episode(
                NOW - 3 * DAY_MS - 3_600_000,
                NOW - 3 * DAY_MS - 60_000,
            ))
            .unwrap();

        assert_eq!(memory_high_days(&store, NOW - 7 * DAY_MS, NOW).unwrap(), 3);
    }

    #[test]
    fn a_memory_diagnostic_with_errors_opens_a_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        store
            .append_batch(&Batch {
                events: vec![WinEvent {
                    ts_ms: NOW - 3 * DAY_MS,
                    channel: Channel::System,
                    provider: "Microsoft-Windows-MemoryDiagnostics-Results".into(),
                    id: 1102,
                    record_id: 7,
                    level: 2,
                    meaning: EventMeaning::MemoryDiagnostic,
                    data_xml: "<UserData><Results><NumPagesTested>1000</NumPagesTested>\
                               <NumPagesUnTested>0</NumPagesUnTested><NumBadPages>3</NumBadPages>\
                               </Results></UserData>"
                        .into(),
                }],
                ..Batch::default()
            })
            .unwrap();
        let mut pass = HealthPass::new(FakeHealth {
            free_gb: 50,
            ..FakeHealth::default()
        });

        let outcome = run(&mut pass, &mut store, NOW, &[]);

        assert_eq!(outcome.critical, 1);
        let kinds: Vec<FindingKind> = store
            .open_findings()
            .unwrap()
            .iter()
            .map(|f| f.finding.kind)
            .collect();
        assert_eq!(kinds, [FindingKind::MemoryTestErrors]);
    }

    #[test]
    fn a_pass_that_cannot_read_events_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(&dir);
        let mut pass = HealthPass::new(FakeHealth {
            free_gb: 4,
            ..FakeHealth::default()
        });
        run(&mut pass, &mut store, NOW, &[]);
        let raw = rusqlite::Connection::open(dir.path().join("vestigium.db")).unwrap();
        raw.execute(
            "INSERT INTO event (ts_ms, channel, provider, event_id, record_id, level, meaning, data_xml)
             VALUES (?1, 99, 'provider', 1, 1, 2, 1, '<EventData/>')",
            [NOW - DAY_MS],
        )
        .unwrap();
        pass.reader.source.free_gb = 50;
        let asked = request(&machine(), NOW + DAY_MS / 24, &[]);

        assert!(matches!(
            pass.run(&mut store, &asked),
            Err(StoreError::UnknownCode {
                what: "channel",
                code: 99
            })
        ));
        let kinds: Vec<FindingKind> = store
            .open_findings()
            .unwrap()
            .iter()
            .map(|f| f.finding.kind)
            .collect();
        assert_eq!(kinds, [FindingKind::LowSpace]);
    }

    /// A source whose disk read blocks until released, noting the time of each pass it is asked for.
    struct Stuck {
        asked: Arc<Mutex<Vec<i64>>>,
        release: Arc<AtomicBool>,
        _alive: Arc<()>,
    }

    impl HealthSource for Stuck {
        fn disk(&mut self, _index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
            lock(&self.asked).push(now_ms);
            while !self.release.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(5));
            }
            Some(DiskHealth {
                name: name.to_string(),
                read_ms: now_ms,
                predict_failure: false,
                attributes: Vec::new(),
                nvme: None,
            })
        }

        fn bus(&mut self, _index: u32) -> Bus {
            Bus::Other
        }

        fn volumes(&mut self) -> Vec<VolumeSpace> {
            vec![VolumeSpace {
                root: r"C:\".into(),
                free_bytes: 4 * GB,
                total_bytes: 100 * GB,
                system: true,
            }]
        }

        fn system_drive(&mut self) -> Option<u32> {
            None
        }

        fn page_files(&mut self) -> Option<Vec<PageFile>> {
            None
        }

        fn antivirus(&mut self) -> Option<Vec<SecurityProduct>> {
            None
        }
    }

    struct Reading {
        dir: tempfile::TempDir,
        service: Service,
        links: Links,
        thread: HealthThread,
        asked: Arc<Mutex<Vec<i64>>>,
        release: Arc<AtomicBool>,
        alive: Arc<()>,
    }

    fn reading(released: bool) -> Reading {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let links = Links::detached();
        let service = Service::start(store, links.clone()).unwrap();
        let asked = Arc::new(Mutex::new(Vec::new()));
        let release = Arc::new(AtomicBool::new(released));
        let alive = Arc::new(());
        let source = Stuck {
            asked: asked.clone(),
            release: release.clone(),
            _alive: alive.clone(),
        };
        let thread = HealthThread::start(
            HealthReader::new(source),
            service.clone(),
            links.status.clone(),
        )
        .unwrap();
        Reading {
            dir,
            service,
            links,
            thread,
            asked,
            release,
            alive,
        }
    }

    impl Reading {
        fn asked(&self) -> Vec<i64> {
            lock(&self.asked).clone()
        }

        fn findings(&self) -> usize {
            Store::open(&self.dir.path().join("vestigium.db"))
                .unwrap()
                .open_findings()
                .unwrap()
                .len()
        }
    }

    const PATIENCE: Duration = Duration::from_secs(15);

    #[test]
    fn a_request_made_while_the_reader_is_stuck_replaces_the_one_waiting() {
        let rig = reading(false);
        rig.thread.request(0, request(&machine(), 1, &[]));
        assert!(wait_until(PATIENCE, || rig.asked() == [1]));

        rig.thread.request(0, request(&machine(), 2, &[]));
        rig.thread.request(0, request(&machine(), 3, &[]));
        rig.release.store(true, Ordering::Release);

        assert!(wait_until(PATIENCE, || rig.asked() == [1, 3]));
        assert!(
            wait_until(PATIENCE, || rig.findings() == 1),
            "the newest pass reached the history"
        );
        assert_eq!(rig.asked(), [1, 3], "the replaced request was never read");
    }

    #[test]
    fn a_source_that_blocks_does_not_delay_the_storage_thread() {
        let rig = reading(false);
        rig.thread.request(0, request(&machine(), 1, &[]));
        assert!(wait_until(PATIENCE, || rig.asked() == [1]));

        let began = std::time::Instant::now();
        rig.service
            .submit(Queued {
                epoch: 0,
                job: Job::Batch(vestigium_store::Batch {
                    bookmark_xml: Some("kept".to_string()),
                    ..Default::default()
                }),
            })
            .ok()
            .unwrap();
        assert!(rig.service.flush(Duration::from_secs(2)));

        assert!(
            began.elapsed() < Duration::from_millis(500),
            "a write waited {:?} behind a source that blocks",
            began.elapsed()
        );
        let store = Store::open(&rig.dir.path().join("vestigium.db")).unwrap();
        assert_eq!(store.bookmark_xml().unwrap().as_deref(), Some("kept"));
        rig.release.store(true, Ordering::Release);
    }

    #[test]
    fn readings_asked_for_before_a_clear_are_dropped() {
        let rig = reading(true);
        let mut pipe = Store::open(&rig.dir.path().join("vestigium.db")).unwrap();
        rig.links.epoch.clear(|| pipe.clear_history()).unwrap();

        rig.thread.request(0, request(&machine(), 1, &[]));
        assert!(wait_until(PATIENCE, || rig.asked() == [1]));
        thread::sleep(Duration::from_millis(300));
        assert!(rig.service.flush(PATIENCE));
        assert_eq!(rig.findings(), 0, "a pass from before the clear is dropped");

        rig.thread.request(1, request(&machine(), 2, &[]));
        assert!(wait_until(PATIENCE, || rig.findings() == 1));
    }

    #[test]
    fn the_critical_findings_come_back_through_the_shared_count() {
        let rig = reading(true);

        rig.thread.request(0, request(&machine(), 1, &[]));

        assert!(wait_until(PATIENCE, || {
            rig.links.critical.load(Ordering::Acquire) == 1
        }));
    }

    #[test]
    fn the_reader_thread_ends_with_its_handle() {
        let Reading { thread, alive, .. } = reading(true);
        assert_eq!(Arc::strong_count(&alive), 2);

        drop(thread);

        assert!(wait_until(PATIENCE, || Arc::strong_count(&alive) == 1));
    }
}
