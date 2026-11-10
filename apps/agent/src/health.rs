use std::collections::{BTreeMap, BTreeSet, HashSet};

use vestigium_collect::{HealthSource, physical_drive};
use vestigium_core::health::{
    Concern, DiskHealth, FindingChange, FindingKind, HealthInputs, SecurityProduct, Subject,
    evaluate, reconcile, unread_kinds,
};
use vestigium_core::hwtest::{Bus, TEST_REACH_DAYS};
use vestigium_core::{EpisodeKind, MachineInfo, Media};
use vestigium_store::{Store, StoreError};

const DAY_MS: i64 = 24 * 60 * 60_000;
/// Event history one pass reads: the 30-day counts and the four weeks a trend compares against.
const EVENT_DAYS: i64 = 35;
/// How often the Security Center is asked; its COM libraries stay loaded after a call.
const ANTIVIRUS_EVERY_MS: i64 = DAY_MS;

/// What the sampler knows at the moment of a pass.
#[derive(Clone, Copy, Debug)]
pub struct PassNow<'a> {
    pub ts_ms: i64,
    pub machine: &'a MachineInfo,
    /// Disks with I/O in the last minute; an idle HDD is not asked, so it never spins up for us.
    pub busy_disks: &'a HashSet<String>,
    pub commit_limit_bytes: u64,
}

/// What one pass did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PassOutcome {
    pub changes: usize,
    /// Findings rated critical after this pass.
    pub critical: u32,
    /// The Security Center was asked, so the pages it touched can be handed back.
    pub asked_security_center: bool,
}

/// Reads the health sources, evaluates the findings and stores what changed.
pub struct HealthPass<S> {
    source: S,
    /// Last reading of each disk, kept while an HDD sleeps.
    disks: BTreeMap<String, DiskHealth>,
    /// The UTC day of each disk's last stored SMART reading.
    smart_day: BTreeMap<String, i64>,
    antivirus: Option<Vec<SecurityProduct>>,
    antivirus_read_ms: Option<i64>,
}

impl<S: HealthSource> HealthPass<S> {
    pub fn new(source: S) -> HealthPass<S> {
        HealthPass {
            source,
            disks: BTreeMap::new(),
            smart_day: BTreeMap::new(),
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

    pub fn run(&mut self, store: &mut Store, now: &PassNow) -> Result<PassOutcome, StoreError> {
        // A failed read is asked again on the next pass; only an answer is kept for a day.
        let asked_security_center = self.antivirus.is_none()
            || self
                .antivirus_read_ms
                .is_none_or(|at| now.ts_ms - at >= ANTIVIRUS_EVERY_MS);
        if asked_security_center {
            self.antivirus = self.source.antivirus();
            self.antivirus_read_ms = self.antivirus.is_some().then_some(now.ts_ms);
        }
        let mut unread: HashSet<&str> = HashSet::new();
        for disk in &now.machine.disks {
            let Some(index) = physical_drive(&disk.name) else {
                continue;
            };
            if self.skips_idle(index, disk.media, now.busy_disks.contains(&disk.name)) {
                unread.insert(&disk.name);
                continue;
            }
            let Some(health) = self.source.disk(index, &disk.name, now.ts_ms) else {
                unread.insert(&disk.name);
                continue;
            };
            let day = health.read_ms.div_euclid(DAY_MS);
            if !health.attributes.is_empty() && self.smart_day.get(&disk.name) != Some(&day) {
                store.record_smart(&health)?;
                self.smart_day.insert(disk.name.clone(), day);
            }
            self.disks.insert(disk.name.clone(), health);
        }
        let in_machine = |name: &String| now.machine.disks.iter().any(|d| &d.name == name);
        self.disks.retain(|name, _| in_machine(name));
        self.smart_day.retain(|name, _| in_machine(name));
        let system_disk = self.source.system_drive().and_then(|number| {
            now.machine
                .disks
                .iter()
                .find(|d| physical_drive(&d.name) == Some(number))
                .map(|d| d.name.clone())
        });

        let week_ago = now.ts_ms - 7 * DAY_MS;
        let open = store.open_findings()?;
        let commit_peak_pct_7d = (now.commit_limit_bytes > 0)
            .then(|| store.minutes(week_ago, now.ts_ms))
            .transpose()?
            .and_then(|minutes| minutes.iter().map(|m| m.commit_max).max())
            .map(|peak| peak as f32 * 100.0 / now.commit_limit_bytes as f32);
        let inputs = HealthInputs {
            now_ms: now.ts_ms,
            history_from_ms: store.first_segment_ms()?.unwrap_or(now.ts_ms),
            machine: now.machine.clone(),
            system_disk,
            disks: self.disks.values().cloned().collect(),
            volumes: self.source.volumes(),
            page_files: self.source.page_files(),
            commit_peak_pct_7d,
            memory_high_days_7d: memory_high_days(store, week_ago, now.ts_ms)?,
            events: store.events(now.ts_ms - EVENT_DAYS * DAY_MS, now.ts_ms)?,
            antivirus: self.antivirus.clone(),
            boot: store.last_scan()?.and_then(|scan| scan.boot),
            tests: store
                .test_runs(now.ts_ms - TEST_REACH_DAYS * DAY_MS, now.ts_ms)?
                .into_iter()
                .map(|row| row.run)
                .collect(),
            open: open
                .iter()
                .map(|s| (s.finding.kind, s.finding.subject.clone()))
                .collect(),
        };
        let fresh = evaluate(&inputs);
        let mut changes = reconcile(&open, &fresh, &unread_kinds(&inputs));
        // A disk this pass did not read keeps its open SMART finding.
        changes.retain(|change| match change {
            FindingChange::Close { id } => !open.iter().any(|s| {
                s.id == *id
                    && s.finding.kind == FindingKind::DiskSmart
                    && matches!(&s.finding.subject, Subject::Disk(name) if unread.contains(name.as_str()))
            }),
            _ => true,
        });
        store.apply_findings(&changes, now.ts_ms)?;
        Ok(PassOutcome {
            changes: changes.len(),
            critical: fresh
                .iter()
                .filter(|f| f.concern == Concern::Critical)
                .count() as u32,
            asked_security_center,
        })
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

    use vestigium_core::health::{PageFile, VolumeSpace};
    use vestigium_core::{Channel, DiskInfo, Episode, EventMeaning, WinEvent};
    use vestigium_store::Batch;

    use super::*;
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
    }

    impl HealthSource for FakeHealth {
        fn disk(&mut self, _index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
            self.asked_disks.borrow_mut().push(name.to_string());
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

    fn run(
        pass: &mut HealthPass<FakeHealth>,
        store: &mut Store,
        at: i64,
        busy: &[&str],
    ) -> PassOutcome {
        run_on(pass, store, &machine(), at, busy)
    }

    fn run_on(
        pass: &mut HealthPass<FakeHealth>,
        store: &mut Store,
        machine: &MachineInfo,
        at: i64,
        busy: &[&str],
    ) -> PassOutcome {
        let busy: HashSet<String> = busy.iter().map(|s| s.to_string()).collect();
        let now = PassNow {
            ts_ms: at,
            machine,
            busy_disks: &busy,
            commit_limit_bytes: 40 * GB,
        };
        pass.run(store, &now).unwrap()
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

        pass.source.free_gb = 50;
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
             VALUES (?1, 1, 'provider', 1, 1, 2, 99, '<EventData/>')",
            [NOW - DAY_MS],
        )
        .unwrap();
        pass.source.free_gb = 50;
        let machine = machine();
        let busy = HashSet::new();
        let now = PassNow {
            ts_ms: NOW + DAY_MS / 24,
            machine: &machine,
            busy_disks: &busy,
            commit_limit_bytes: 40 * GB,
        };

        assert!(matches!(
            pass.run(&mut store, &now),
            Err(StoreError::UnknownCode {
                what: "event meaning",
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
}
