use std::collections::{BTreeMap, BTreeSet, HashSet};

use vestigium_collect::{HealthSource, physical_drive};
use vestigium_core::health::{
    Concern, DiskHealth, HealthInputs, SecurityProduct, evaluate, reconcile,
};
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
    pub critical: bool,
    /// The Security Center was asked, so the pages it touched can be handed back.
    pub asked_security_center: bool,
}

/// Reads the health sources, evaluates the findings and stores what changed.
pub struct HealthPass<S> {
    source: S,
    /// Last reading of each disk, kept while an HDD sleeps.
    disks: BTreeMap<String, DiskHealth>,
    antivirus: Option<Vec<SecurityProduct>>,
    antivirus_read_ms: Option<i64>,
}

impl<S: HealthSource> HealthPass<S> {
    pub fn new(source: S) -> HealthPass<S> {
        HealthPass {
            source,
            disks: BTreeMap::new(),
            antivirus: None,
            antivirus_read_ms: None,
        }
    }

    pub fn run(&mut self, store: &mut Store, now: &PassNow) -> Result<PassOutcome, StoreError> {
        let asked_security_center = self
            .antivirus_read_ms
            .is_none_or(|at| now.ts_ms - at >= ANTIVIRUS_EVERY_MS);
        if asked_security_center {
            self.antivirus = self.source.antivirus();
            self.antivirus_read_ms = Some(now.ts_ms);
        }
        for disk in &now.machine.disks {
            let Some(index) = physical_drive(&disk.name) else {
                continue;
            };
            if disk.media == Media::Hdd && !now.busy_disks.contains(&disk.name) {
                continue;
            }
            if let Some(health) = self.source.disk(index, &disk.name, now.ts_ms) {
                self.disks.insert(disk.name.clone(), health);
            }
        }
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
            open: open
                .iter()
                .map(|s| (s.finding.kind, s.finding.subject.clone()))
                .collect(),
        };
        let fresh = evaluate(&inputs);
        let changes = reconcile(&open, &fresh);
        store.apply_findings(&changes, now.ts_ms)?;
        Ok(PassOutcome {
            changes: changes.len(),
            critical: fresh.iter().any(|f| f.concern == Concern::Critical),
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
    use std::cell::RefCell;
    use std::rc::Rc;

    use vestigium_core::health::{FindingKind, PageFile, VolumeSpace};
    use vestigium_core::{DiskInfo, Episode};

    use super::*;

    const GB: u64 = 1 << 30;
    const NOW: i64 = 100 * DAY_MS;

    /// A machine whose answers the test sets, counting what the pass asked.
    #[derive(Clone, Default)]
    struct FakeHealth {
        free_gb: u64,
        asked_disks: Rc<RefCell<Vec<String>>>,
        asked_antivirus: Rc<RefCell<u32>>,
        failing_disk: bool,
    }

    impl HealthSource for FakeHealth {
        fn disk(&mut self, _index: u32, name: &str, now_ms: i64) -> Option<DiskHealth> {
            self.asked_disks.borrow_mut().push(name.to_string());
            Some(DiskHealth {
                name: name.to_string(),
                read_ms: now_ms,
                predict_failure: self.failing_disk,
                attributes: Vec::new(),
                nvme: None,
            })
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
            Some(vec![PageFile {
                path: r"C:\pagefile.sys".into(),
                size_bytes: 8 * GB,
                peak_bytes: GB,
            }])
        }

        fn antivirus(&mut self) -> Option<Vec<SecurityProduct>> {
            *self.asked_antivirus.borrow_mut() += 1;
            Some(vec![SecurityProduct {
                name: "Built-in antivirus".into(),
                on: true,
            }])
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
        let machine = machine();
        let busy: HashSet<String> = busy.iter().map(|s| s.to_string()).collect();
        let now = PassNow {
            ts_ms: at,
            machine: &machine,
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
        assert!(first.critical);
        assert_eq!(
            store.open_findings().unwrap()[0].finding.kind,
            FindingKind::LowSpace
        );

        pass.source.free_gb = 50;
        let second = run(&mut pass, &mut store, NOW + DAY_MS / 24, &[]);
        assert_eq!(second.changes, 1);
        assert!(!second.critical);
        assert!(store.open_findings().unwrap().is_empty());
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
}
