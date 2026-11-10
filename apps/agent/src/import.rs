use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use vestigium_core::health::{SCAN_VERSION, ScanResult, Subject};
use vestigium_core::hwtest::{TEST_RUN_VERSION, TestRun, TestRunV1, disk_subject};
use vestigium_core::{
    Applier, CAPTURE_VERSION, CAPTURE_VERSION_MIN, DUMP_DELETION_VERSION, DUMP_SCAN_VERSION,
    DeepCapture, DumpDeletion, DumpScan, ProbeHiccup, TWEAK_PLAN_VERSION, TweakResults,
};
use vestigium_store::Store;

/// Stores the finished capture files of `dir` and deletes them; returns the ids in name order.
/// A file that does not decode is kept, since the helper writes it only when its capture ends.
pub fn import_captures(dir: &Path, hiccups: &[ProbeHiccup], store: &mut Store) -> Vec<i64> {
    let mut ids = Vec::new();
    for path in numbered_files(dir, "capture") {
        let Some(mut capture) = read_capture(&path) else {
            continue;
        };
        capture.hiccups.extend(
            hiccups
                .iter()
                .filter(|h| (capture.start_ms..=capture.end_ms).contains(&h.ts_ms))
                .copied(),
        );
        let Ok(id) = store.insert_capture(&capture) else {
            continue;
        };
        ids.push(id);
        let _ = fs::remove_file(&path);
    }
    ids
}

/// Stores the health scan files of `dir` and deletes them; returns the ids in name order.
pub fn import_scans(dir: &Path, store: &mut Store) -> Vec<i64> {
    let mut ids = Vec::new();
    for path in numbered_files(dir, "scan") {
        let Some(scan) = read_versioned::<ScanResult>(&path, SCAN_VERSION) else {
            continue;
        };
        let Ok(id) = store.insert_scan(&scan) else {
            continue;
        };
        ids.push(id);
        let _ = fs::remove_file(&path);
    }
    ids
}

/// Stores the dump reads of `dir` and applies its deletion results, deleting both files and the
/// lists they answer; returns the dumps stored and how many were marked as deleted.
pub fn import_dumps(dir: &Path, store: &mut Store) -> (Vec<i64>, u32) {
    let mut ids = Vec::new();
    for path in numbered_files(dir, "dumps") {
        let Some(scan) = read_versioned::<DumpScan>(&path, DUMP_SCAN_VERSION) else {
            continue;
        };
        let Ok(stored) = store.upsert_dumps(&scan.dumps, scan.ts_ms) else {
            continue;
        };
        ids.extend(stored);
        let _ = fs::remove_file(&path);
    }
    let mut deleted = 0;
    for path in numbered_files(dir, "deleted") {
        let Some(done) = read_versioned::<DumpDeletion>(&path, DUMP_DELETION_VERSION) else {
            continue;
        };
        let Ok(present) = store.present_dumps() else {
            continue;
        };
        let gone: Vec<i64> = present
            .iter()
            .filter(|row| {
                done.deleted
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(&row.record.path))
            })
            .map(|row| row.id)
            .collect();
        let Ok(marked) = store.mark_dumps_deleted(&gone, done.ts_ms) else {
            continue;
        };
        deleted += marked as u32;
        let _ = fs::remove_file(path.with_extension("delete"));
        let _ = fs::remove_file(&path);
    }
    (ids, deleted)
}

/// Stores the tweak results of `dir` in the history and deletes them with their plans; returns
/// the ids of the changes in the order read.
pub fn import_tweaks(dir: &Path, store: &mut Store) -> Vec<i64> {
    let mut ids = Vec::new();
    for path in numbered_files(dir, "tweaked") {
        let Some(results) = read_versioned::<TweakResults>(&path, TWEAK_PLAN_VERSION) else {
            continue;
        };
        let stored: Result<Vec<i64>, _> = results
            .changes
            .iter()
            .map(|change| store.record_tweak_change(change, results.ts_ms, Applier::Elevated))
            .collect();
        let Ok(stored) = stored else {
            continue;
        };
        ids.extend(stored);
        let _ = fs::remove_file(path.with_extension("plan"));
        let _ = fs::remove_file(&path);
    }
    ids
}

/// Larger than any test result; such a file is not one and stays where it is.
const MAX_TEST_FILE: u64 = 1 << 20;
/// What the window (`.test`) and the helper (`.surface`, `.chkdsk`) write when a test ends.
const TEST_FILES: [&str; 3] = ["test", "surface", "chkdsk"];

/// Stores the test results of `dir` and deletes them, with what a disk scan left beside its
/// result; a disk known by number gets the name the disk counters give it. Returns the ids.
pub fn import_tests(dir: &Path, store: &mut Store) -> Vec<i64> {
    let disks = store
        .machine()
        .ok()
        .flatten()
        .map(|m| m.disks)
        .unwrap_or_default();
    let mut ids = Vec::new();
    for extension in TEST_FILES {
        for path in numbered_files(dir, extension) {
            if fs::metadata(&path).map_or(true, |m| m.len() > MAX_TEST_FILE) {
                continue;
            }
            let Some(mut run) = read_test_run(&path) else {
                continue;
            };
            if let Subject::Disk(name) = &run.subject
                && let Ok(number) = name.parse::<u32>()
            {
                run.subject = disk_subject(number, &disks);
            }
            let Ok(id) = store.record_test_run(&run) else {
                continue;
            };
            ids.push(id);
            let _ = fs::remove_file(path.with_extension("progress"));
            let _ = fs::remove_file(path.with_extension("stop"));
            let _ = fs::remove_file(&path);
        }
    }
    ids
}

/// How long an empty result, or the temporary file of one, may sit unchanged before it counts as
/// what a helper that died left behind; longer than any job, the deep capture's 24 hours included.
const LEFTOVER_AFTER: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// The files named `<digits>.<extension>`, in name order. An empty one, or a `<digits>.<extension>.tmp`,
/// untouched for `LEFTOVER_AFTER` is deleted instead of listed.
fn numbered_files(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let now = SystemTime::now();
    let temporary = format!("{extension}.tmp");
    let mut files = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        if is_numbered(&path, &temporary) {
            if is_left_over(&entry, now, false) {
                let _ = fs::remove_file(&path);
            }
        } else if is_numbered(&path, extension) {
            if is_left_over(&entry, now, true) {
                let _ = fs::remove_file(&path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Whether the file has not changed for `LEFTOVER_AFTER` and, when `only_empty`, holds nothing.
fn is_left_over(entry: &fs::DirEntry, now: SystemTime, only_empty: bool) -> bool {
    let Ok(meta) = entry.metadata() else {
        return false;
    };
    (!only_empty || meta.len() == 0)
        && meta
            .modified()
            .ok()
            .and_then(|at| now.duration_since(at).ok())
            .is_some_and(|age| age >= LEFTOVER_AFTER)
}

fn is_numbered(path: &Path, extension: &str) -> bool {
    let digits = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(&format!(".{extension}")));
    digits.is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// A capture file of any format version still read.
fn read_capture(path: &Path) -> Option<DeepCapture> {
    let bytes = fs::read(path).ok()?;
    let (&version, body) = bytes.split_first()?;
    if !(CAPTURE_VERSION_MIN..=CAPTURE_VERSION).contains(&version) {
        return None;
    }
    postcard::from_bytes(body).ok()
}

/// A test file of the current layout or of version 1, which has no dirty state in a volume check.
fn read_test_run(path: &Path) -> Option<TestRun> {
    read_versioned::<TestRun>(path, TEST_RUN_VERSION)
        .or_else(|| read_versioned::<TestRunV1>(path, 1).map(TestRun::from))
}

/// A file written as one version byte and a postcard body; `None` for another version.
fn read_versioned<T: serde::de::DeserializeOwned>(path: &Path, expected: u8) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    let (&version, body) = bytes.split_first()?;
    if version != expected {
        return None;
    }
    postcard::from_bytes(body).ok()
}

#[cfg(test)]
mod tests {
    use vestigium_core::CaptureSource;

    use super::*;

    fn capture(start_ms: i64, end_ms: i64) -> DeepCapture {
        DeepCapture {
            start_ms,
            end_ms,
            kernel: CaptureSource::Ok,
            frames: CaptureSource::Ok,
            lost_events: 0,
            drivers: Vec::new(),
            longest: Vec::new(),
            apps: Vec::new(),
            hitches: Vec::new(),
            hiccups: Vec::new(),
        }
    }

    fn write_capture(dir: &Path, name: &str, capture: &DeepCapture) {
        let mut bytes = vec![CAPTURE_VERSION];
        bytes.extend(postcard::to_allocvec(capture).unwrap());
        fs::write(dir.join(name), bytes).unwrap();
    }

    fn store(dir: &Path) -> Store {
        Store::open(&dir.join("vestigium.db")).unwrap()
    }

    fn hiccup(ts_ms: i64) -> ProbeHiccup {
        ProbeHiccup {
            ts_ms,
            lag_ms: 12.0,
        }
    }

    #[test]
    fn import_reads_only_numbered_capture_files() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        write_capture(&captures, "20.capture", &capture(2_000, 3_000));
        write_capture(&captures, "10.capture", &capture(1_000, 1_500));
        write_capture(&captures, "notes.capture", &capture(5_000, 6_000));
        write_capture(&captures, "30.capture.tmp", &capture(7_000, 8_000));
        write_capture(&captures, "40.etl", &capture(9_000, 9_500));
        write_capture(&captures, ".capture", &capture(9_000, 9_500));
        fs::create_dir(captures.join("50.capture")).unwrap();
        let mut store = store(dir.path());

        let ids = import_captures(&captures, &[], &mut store);

        let stored: Vec<i64> = store
            .captures(i64::MIN, i64::MAX)
            .unwrap()
            .iter()
            .map(|c| c.start_ms)
            .collect();
        assert_eq!(ids.len(), 2);
        assert_eq!(stored, [1_000, 2_000], "read in name order");
        assert!(!captures.join("10.capture").exists());
        assert!(!captures.join("20.capture").exists());
        for kept in [
            "notes.capture",
            "30.capture.tmp",
            "40.etl",
            ".capture",
            "50.capture",
        ] {
            assert!(captures.join(kept).exists(), "{kept} is not ours to take");
        }
    }

    #[test]
    fn import_reads_a_file_of_the_first_format() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        let mut bytes = vec![CAPTURE_VERSION_MIN];
        bytes.extend(postcard::to_allocvec(&capture(1_000, 2_000)).unwrap());
        fs::write(captures.join("1.capture"), bytes).unwrap();
        let mut store = store(dir.path());

        let ids = import_captures(&captures, &[], &mut store);

        assert_eq!(ids.len(), 1);
        assert!(!captures.join("1.capture").exists());
    }

    #[test]
    fn import_keeps_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        fs::write(captures.join("1.capture"), b"").unwrap();
        fs::write(captures.join("2.capture"), [CAPTURE_VERSION + 1, 0, 1, 2]).unwrap();
        fs::write(
            captures.join("4.capture"),
            [CAPTURE_VERSION_MIN - 1, 0, 1, 2],
        )
        .unwrap();
        fs::write(captures.join("3.capture"), [CAPTURE_VERSION, 0xff, 0xff]).unwrap();
        let mut store = store(dir.path());

        let ids = import_captures(&captures, &[], &mut store);

        assert!(ids.is_empty());
        assert!(store.captures(i64::MIN, i64::MAX).unwrap().is_empty());
        for name in ["1.capture", "2.capture", "3.capture", "4.capture"] {
            assert!(captures.join(name).exists(), "{name} may still be written");
        }
    }

    #[test]
    fn import_adds_the_hiccups_of_its_window() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        write_capture(&captures, "1.capture", &capture(1_000, 2_000));
        let mut store = store(dir.path());
        let seen = [
            hiccup(999),
            hiccup(1_000),
            hiccup(1_500),
            hiccup(2_000),
            hiccup(2_001),
        ];

        let ids = import_captures(&captures, &seen, &mut store);

        let stored = store.capture(ids[0]).unwrap().unwrap();
        let times: Vec<i64> = stored.hiccups.iter().map(|h| h.ts_ms).collect();
        assert_eq!(times, [1_000, 1_500, 2_000]);
    }

    #[test]
    fn import_twice_is_one_row() {
        let dir = tempfile::tempdir().unwrap();
        let captures = dir.path().join("captures");
        fs::create_dir(&captures).unwrap();
        write_capture(&captures, "1.capture", &capture(1_000, 2_000));
        let mut store = store(dir.path());

        let first = import_captures(&captures, &[], &mut store);
        let second = import_captures(&captures, &[], &mut store);

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(store.captures(i64::MIN, i64::MAX).unwrap().len(), 1);
    }

    fn write_versioned<T: serde::Serialize>(path: &Path, version: u8, value: &T) {
        let mut bytes = vec![version];
        bytes.extend(postcard::to_allocvec(value).unwrap());
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn dump_reads_and_deletions_are_imported_once() {
        use vestigium_core::{DumpKind, DumpRecord};

        let dir = tempfile::tempdir().unwrap();
        let dumps = dir.path().join("dumps");
        fs::create_dir(&dumps).unwrap();
        let mini = DumpRecord {
            path: r"C:\Windows\Minidump\1.dmp".into(),
            file_ms: 5_000,
            kind: DumpKind::KernelMini,
            crash_ms: 4_000,
            size_bytes: 2048,
            analysis: None,
        };
        write_versioned(
            &dumps.join("10.dumps"),
            DUMP_SCAN_VERSION,
            &DumpScan {
                ts_ms: 10,
                dumps: vec![mini],
            },
        );
        let mut store = store(dir.path());

        let (ids, deleted) = import_dumps(&dumps, &mut store);
        assert_eq!((ids.len(), deleted), (1, 0));
        assert_eq!(
            import_dumps(&dumps, &mut store),
            (Vec::new(), 0),
            "the file is gone"
        );

        fs::write(dumps.join("20.delete"), r"C:\Windows\Minidump\1.dmp").unwrap();
        write_versioned(
            &dumps.join("20.deleted"),
            DUMP_DELETION_VERSION,
            &DumpDeletion {
                ts_ms: 20,
                deleted: vec![r"c:\windows\minidump\1.DMP".into()],
                refused: Vec::new(),
                failed: Vec::new(),
            },
        );
        assert_eq!(import_dumps(&dumps, &mut store), (Vec::new(), 1));
        assert!(store.present_dumps().unwrap().is_empty());
        assert!(
            !dumps.join("20.delete").exists(),
            "the list it answered is cleaned up"
        );
    }

    #[test]
    fn imports_a_tweaked_file_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());
        let tweaks = dir.path().join("tweaks");
        fs::create_dir(&tweaks).unwrap();
        let mpo = vestigium_core::TweakId::Mpo;
        let change = vestigium_core::TweakChange {
            tweak: mpo,
            subject: None,
            undo_of: None,
            results: vec![vestigium_core::SettingResult {
                change: vestigium_core::SettingChange {
                    setting: mpo.settings()[0].clone(),
                    from: vestigium_core::Value::Absent,
                    to: vestigium_core::Value::Dword(5),
                },
                error: None,
            }],
            restore_point: Some(vestigium_core::RestorePoint::Created),
        };
        fs::write(tweaks.join("30.plan"), [TWEAK_PLAN_VERSION]).unwrap();
        write_versioned(
            &tweaks.join("30.tweaked"),
            TWEAK_PLAN_VERSION,
            &TweakResults {
                ts_ms: 30,
                changes: vec![change.clone()],
            },
        );

        assert_eq!(import_tweaks(&tweaks, &mut store), vec![1]);
        assert!(import_tweaks(&tweaks, &mut store).is_empty());
        assert!(!tweaks.join("30.plan").exists() && !tweaks.join("30.tweaked").exists());
        let rows = store.tweaks_in_place().unwrap();
        assert_eq!(rows[0].change, change);
        assert_eq!(rows[0].applier, Applier::Elevated);
    }

    fn age(path: &Path, by: Duration) {
        let file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - by).unwrap();
    }

    #[test]
    fn an_old_empty_result_is_a_leftover_and_a_fresh_one_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());
        let tweaks = dir.path().join("tweaks");
        fs::create_dir(&tweaks).unwrap();
        fs::write(tweaks.join("30.tweaked"), b"").unwrap();
        fs::write(tweaks.join("31.tweaked"), b"").unwrap();
        fs::write(tweaks.join("32.tweaked"), [TWEAK_PLAN_VERSION, 0xff, 0xff]).unwrap();
        age(
            &tweaks.join("30.tweaked"),
            LEFTOVER_AFTER + Duration::from_secs(60),
        );
        age(
            &tweaks.join("32.tweaked"),
            LEFTOVER_AFTER + Duration::from_secs(60),
        );

        assert!(import_tweaks(&tweaks, &mut store).is_empty());

        assert!(!tweaks.join("30.tweaked").exists(), "old and empty goes");
        assert!(
            tweaks.join("31.tweaked").exists(),
            "a helper may be running"
        );
        assert!(
            tweaks.join("32.tweaked").exists(),
            "an unreadable file stays"
        );
    }

    #[test]
    fn an_old_temporary_result_is_deleted_and_never_imported() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());
        let tweaks = dir.path().join("tweaks");
        fs::create_dir(&tweaks).unwrap();
        fs::write(tweaks.join("40.tweaked.tmp"), [TWEAK_PLAN_VERSION, 1]).unwrap();
        fs::write(tweaks.join("41.tweaked.tmp"), [TWEAK_PLAN_VERSION, 1]).unwrap();
        age(
            &tweaks.join("40.tweaked.tmp"),
            LEFTOVER_AFTER + Duration::from_secs(60),
        );

        assert!(import_tweaks(&tweaks, &mut store).is_empty());

        assert!(!tweaks.join("40.tweaked.tmp").exists());
        assert!(tweaks.join("41.tweaked.tmp").exists());
    }

    #[test]
    fn import_without_a_folder_stores_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());

        assert!(import_captures(&dir.path().join("missing"), &[], &mut store).is_empty());
    }

    fn surface_run(disk: u32) -> TestRun {
        let scan = vestigium_core::hwtest::SurfaceAggregator::new(
            disk,
            vestigium_core::Media::Hdd,
            1 << 30,
            512,
            0,
        )
        .finish(1 << 30, None);
        TestRun {
            subject: Subject::Disk(disk.to_string()),
            started_ms: 10,
            ended_ms: 20,
            outcome: vestigium_core::hwtest::Outcome::Clean,
            data: vestigium_core::hwtest::TestData::DiskSurface(scan),
        }
    }

    #[test]
    fn imports_test_results_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());
        store
            .set_machine(&vestigium_core::MachineInfo {
                logical_cpus: 8,
                mem_total_bytes: 16 << 30,
                disks: vec![vestigium_core::DiskInfo {
                    name: "1 D:".into(),
                    media: vestigium_core::Media::Hdd,
                }],
            })
            .unwrap();
        let tests = dir.path().join("tests");
        fs::create_dir(&tests).unwrap();
        write_versioned(&tests.join("40.surface"), TEST_RUN_VERSION, &surface_run(1));
        fs::write(tests.join("40.stop"), b"").unwrap();
        fs::write(tests.join("41.chkdsk"), [99u8, 1, 2]).unwrap();
        fs::write(tests.join("note.test"), [TEST_RUN_VERSION]).unwrap();

        assert_eq!(import_tests(&tests, &mut store), vec![1]);
        assert!(import_tests(&tests, &mut store).is_empty());
        assert!(!tests.join("40.surface").exists() && !tests.join("40.stop").exists());
        assert!(tests.join("41.chkdsk").exists(), "another version stays");
        let runs = store.test_runs(0, 100).unwrap();
        assert_eq!(runs[0].run.subject, Subject::Disk("1 D:".into()));
    }

    #[test]
    fn a_volume_check_file_of_layout_version_one_is_imported() {
        use vestigium_core::hwtest::{Outcome, TestDataV1, TestRunV1, VolumeCheckV1, VolumeResult};
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());
        let tests = dir.path().join("tests");
        fs::create_dir(&tests).unwrap();
        let old = TestRunV1 {
            subject: Subject::Volume(r"D:\".into()),
            started_ms: 10,
            ended_ms: 20,
            outcome: Outcome::Clean,
            data: TestDataV1::VolumeCheck(VolumeCheckV1 {
                volume: r"D:\".into(),
                exit_code: Some(0),
                result: VolumeResult::NoProblems,
                output: String::new(),
            }),
        };
        write_versioned(&tests.join("60.chkdsk"), 1, &old);
        assert_eq!(import_tests(&tests, &mut store).len(), 1);
        assert!(!tests.join("60.chkdsk").exists());
        assert_eq!(store.test_runs(0, 100).unwrap().len(), 1);
    }

    #[test]
    fn a_test_file_too_large_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = store(dir.path());
        let tests = dir.path().join("tests");
        fs::create_dir(&tests).unwrap();
        let mut bytes = vec![TEST_RUN_VERSION];
        bytes.resize(MAX_TEST_FILE as usize + 1, 0);
        fs::write(tests.join("50.test"), bytes).unwrap();
        assert!(import_tests(&tests, &mut store).is_empty());
        assert!(tests.join("50.test").exists());
    }
}
