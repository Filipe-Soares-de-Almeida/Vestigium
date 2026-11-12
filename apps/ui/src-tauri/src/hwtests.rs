use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use vestigium_collect::{
    BenchTarget, benchmark, disk_bus, disk_health, disk_media, disk_nvme, disk_size, file_system,
    memory_diagnostic_events, memory_info, memory_test, physical_drive, volume_drive, volumes,
};
use vestigium_core::health::{DiskHealth, NvmeDetail, Subject};
use vestigium_core::hwtest::{
    BENCH_FILE_BYTES, BENCH_SECONDS, BenchMode, Bus, MemoryDiagnostic, Pattern, QUICK_MINUTES,
    SurfaceProgress, TEST_RUN_VERSION, TestKind, TestRun, bench_reference, memory_diagnostics,
    quick_test_size,
};
use vestigium_core::{MachineInfo, Media};

use crate::agent::UiError;
use crate::capture::Ended;

/// How far back the Tests page lists earlier runs.
pub const RUNS_SHOWN_MS: i64 = 365 * 24 * 3_600_000;
/// A disk scan whose progress file is older than this has stopped without a result.
const PROGRESS_STALE_MS: i64 = 60_000;

/// Memory now, and how much a quick test would take.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MemoryDto {
    pub total_bytes: u64,
    pub available_bytes: u64,
    /// `None` when too little is free for a quick test.
    pub quick_bytes: Option<u64>,
    pub minutes: [u32; 3],
}

/// One physical disk the tests can read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiskDto {
    pub number: u32,
    /// As the disk counters name it, such as "0 C:", or the bare number.
    pub name: String,
    pub media: Media,
    pub bus: Bus,
    pub bytes: Option<u64>,
    /// Roots of the volumes on it, such as "C:\".
    pub volumes: Vec<String>,
    /// The MB/s typical of a disk of this kind, per benchmark mode; none where disks vary too much.
    pub ranges: Vec<BenchRange>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct BenchRange {
    pub mode: BenchMode,
    pub low: f32,
    pub high: f32,
}

/// One fixed volume the benchmark and the file system check can use.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VolumeDto {
    pub root: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub system: bool,
    pub disk: Option<u32>,
    pub file_system: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TestRunDto {
    pub id: i64,
    #[serde(flatten)]
    pub run: TestRun,
}

/// A disk scan the helper is running, known from the progress file it rewrites.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScanUnderWay {
    /// The digits that name its files.
    pub stem: String,
    pub progress: SurfaceProgress,
}

/// Everything the Tests page shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TestsDto {
    pub memory: MemoryDto,
    /// Runs of the Windows Memory Diagnostic still in the System log, newest first.
    pub diagnostics: Vec<MemoryDiagnostic>,
    pub disks: Vec<DiskDto>,
    pub volumes: Vec<VolumeDto>,
    /// Earlier runs, oldest first.
    pub runs: Vec<TestRunDto>,
    pub running: Option<RunningTest>,
    pub scan: Option<ScanUnderWay>,
    pub bench_file_bytes: u64,
    /// Free space a volume needs before the speed test may start: the file and a margin.
    pub bench_needed_bytes: u64,
    /// When this Windows session began, as wall-clock milliseconds: a repair a check queued runs
    /// at the next boot, so a check older than this no longer leaves a volume marked.
    pub boot_ms: i64,
}

/// SMART or NVMe detail of one disk, read when asked: reading an idle HDD spins it up.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiskDetailDto {
    pub health: Option<DiskHealth>,
    pub nvme: Option<NvmeDetail>,
}

/// A test this window started, while it runs.
#[derive(Clone, Debug, Serialize)]
pub struct RunningTest {
    pub kind: TestKind,
    pub subject: Subject,
    pub started_ms: i64,
    /// When a test of fixed length ends.
    pub ends_ms: Option<i64>,
    #[serde(skip)]
    pub stop: Arc<AtomicBool>,
    /// The file whose presence stops a disk scan in the helper.
    #[serde(skip)]
    pub stop_file: Option<PathBuf>,
}

impl PartialEq for RunningTest {
    fn eq(&self, other: &RunningTest) -> bool {
        (self.kind, &self.subject, self.started_ms, self.ends_ms)
            == (other.kind, &other.subject, other.started_ms, other.ends_ms)
    }
}

/// What the window hears while a test runs.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TestProgress {
    Memory {
        pattern: Pattern,
        rounds: u32,
        passes: u32,
        error_count: u64,
    },
    Benchmark {
        mode: BenchMode,
    },
    Surface(SurfaceProgress),
}

/// What the window hears when a test ends.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TestFinished {
    /// The run as stored; `None` while the watcher is stopped, until it imports the file.
    pub id: Option<i64>,
    pub run: Option<TestRun>,
    pub exit_code: Option<u32>,
    pub ended: Ended,
}

pub fn memory_now() -> MemoryDto {
    let (total, available) = memory_info().map_or((0, 0), |m| (m.total_bytes, m.avail_bytes));
    MemoryDto {
        total_bytes: total,
        available_bytes: available,
        quick_bytes: quick_test_size(total, available),
        minutes: QUICK_MINUTES,
    }
}

/// Every Memory Diagnostic run the System log still holds, newest first.
pub fn diagnostics() -> Vec<MemoryDiagnostic> {
    let mut runs = memory_diagnostics(&memory_diagnostic_events().unwrap_or_default());
    runs.reverse();
    runs
}

/// The fixed volumes with the disk under each.
pub fn volume_list() -> Vec<VolumeDto> {
    volumes()
        .into_iter()
        .map(|v| VolumeDto {
            disk: volume_drive(&v.root),
            file_system: file_system(&v.root),
            root: v.root,
            total_bytes: v.total_bytes,
            free_bytes: v.free_bytes,
            system: v.system,
        })
        .collect()
}

/// The physical disks the watcher knows, plus any under a volume it does not.
pub fn disk_list(machine: Option<&MachineInfo>, volumes: &[VolumeDto]) -> Vec<DiskDto> {
    let mut numbers: Vec<(u32, String)> = machine
        .map(|m| {
            m.disks
                .iter()
                .filter_map(|d| Some((physical_drive(&d.name)?, d.name.clone())))
                .collect()
        })
        .unwrap_or_default();
    for disk in volumes.iter().filter_map(|v| v.disk) {
        if !numbers.iter().any(|(n, _)| *n == disk) {
            numbers.push((disk, disk.to_string()));
        }
    }
    numbers.sort();
    numbers
        .into_iter()
        .map(|(number, name)| (number, name, disk_media(number), disk_bus(number)))
        .map(|(number, name, media, bus)| DiskDto {
            number,
            name,
            media,
            bus,
            ranges: BenchMode::ALL
                .into_iter()
                .filter_map(|mode| {
                    bench_reference(bus, media, mode).map(|(low, high)| BenchRange {
                        mode,
                        low,
                        high,
                    })
                })
                .collect(),
            bytes: disk_size(number),
            volumes: volumes
                .iter()
                .filter(|v| v.disk == Some(number))
                .map(|v| v.root.clone())
                .collect(),
        })
        .collect()
}

pub fn disk_detail(number: u32, name: &str, now_ms: i64) -> DiskDetailDto {
    DiskDetailDto {
        health: disk_health(number, name, now_ms),
        nvme: disk_nvme(number),
    }
}

/// `<data>\tests\<ms>.<extension>`, beside the watcher's database.
pub fn tests_file(db: &Path, started_ms: i64, extension: &str) -> Option<PathBuf> {
    Some(
        db.parent()?
            .join("tests")
            .join(format!("{started_ms}.{extension}")),
    )
}

pub fn disk_scan_args(disk: u32, from_mib: u64, out: &Path) -> String {
    let from = if from_mib > 0 {
        format!(" --from {from_mib}")
    } else {
        String::new()
    };
    format!("disk-scan --disk {disk} --out \"{}\"{from}", out.display())
}

pub fn volume_check_args(letter: char, out: &Path) -> String {
    format!(
        "volume-check --volume {letter}: --out \"{}\"",
        out.display()
    )
}

/// The drive letter of a root such as "D:\", when it is one.
pub fn letter_of(root: &str) -> Option<char> {
    let mut chars = root.chars();
    let letter = chars.next().filter(char::is_ascii_alphabetic)?;
    (chars.next() == Some(':')).then(|| letter.to_ascii_uppercase())
}

/// A file the window writes as the agent reads it: the version byte, then the body.
pub fn write_versioned(path: &Path, run: &TestRun) -> Result<(), UiError> {
    let mut bytes = vec![TEST_RUN_VERSION];
    bytes.extend(postcard::to_allocvec(run).map_err(|e| UiError::Failed(e.to_string()))?);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| UiError::Failed(e.to_string()))?;
    }
    std::fs::write(path, bytes).map_err(|e| UiError::Failed(e.to_string()))
}

pub fn read_progress(path: &Path) -> Option<SurfaceProgress> {
    let bytes = std::fs::read(path).ok()?;
    let (&version, body) = bytes.split_first()?;
    (version == TEST_RUN_VERSION)
        .then(|| postcard::from_bytes(body).ok())
        .flatten()
}

/// The file whose appearance ends the disk scan that writes `<stem>.surface`.
pub fn stop_file_of(tests_dir: &Path, stem: &str) -> PathBuf {
    tests_dir.join(format!("{stem}.stop"))
}

/// A disk scan the helper still runs: a fresh progress file with no result in its `.surface` yet.
pub fn scan_under_way(tests_dir: &Path, now_ms: i64) -> Option<ScanUnderWay> {
    let entries = std::fs::read_dir(tests_dir).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "progress"))
        .filter(|p| std::fs::metadata(p.with_extension("surface")).map_or(true, |m| m.len() == 0))
        .filter_map(|p| {
            let progress = read_progress(&p)?;
            let stem = p.file_stem()?.to_str()?.to_string();
            (now_ms - progress.ts_ms <= PROGRESS_STALE_MS)
                .then_some(ScanUnderWay { stem, progress })
        })
        .max_by_key(|s| s.progress.ts_ms)
}

/// The folder for the benchmark's file: the temporary folder when it is on that volume, the
/// volume's root otherwise.
pub fn bench_dir(root: &str) -> PathBuf {
    let temp = std::env::temp_dir();
    let same = temp
        .to_str()
        .and_then(letter_of)
        .zip(letter_of(root))
        .is_some_and(|(a, b)| a == b);
    if same { temp } else { PathBuf::from(root) }
}

pub fn bench_target(volume: &VolumeDto) -> BenchTarget {
    let disk = volume.disk;
    BenchTarget {
        dir: bench_dir(&volume.root),
        volume: volume.root.clone(),
        disk,
        bus: disk.map_or(Bus::Other, disk_bus),
        media: disk.map_or(Media::Unknown, disk_media),
    }
}

/// Runs a quick memory test of `minutes` on this thread.
pub fn run_memory_test(
    minutes: u32,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(TestProgress),
) -> Result<TestRun, UiError> {
    let bytes = memory_now()
        .quick_bytes
        .ok_or_else(|| UiError::Failed("too little free memory for a quick test".into()))?;
    Ok(memory_test(
        bytes,
        Duration::from_secs(u64::from(minutes) * 60),
        stop,
        &mut |p| {
            progress(TestProgress::Memory {
                pattern: p.pattern,
                rounds: p.rounds,
                passes: p.passes,
                error_count: p.error_count,
            })
        },
    ))
}

pub fn run_benchmark(
    volume: &VolumeDto,
    stop: &AtomicBool,
    progress: &mut dyn FnMut(TestProgress),
) -> TestRun {
    benchmark(
        &bench_target(volume),
        BENCH_FILE_BYTES,
        BENCH_SECONDS,
        stop,
        &mut |mode| progress(TestProgress::Benchmark { mode }),
    )
}

/// Sets the stop of a test running in this window, or writes the file that stops a disk scan.
pub fn stop(running: &RunningTest) {
    running.stop.store(true, Ordering::Release);
    if let Some(file) = &running.stop_file {
        let _ = std::fs::write(file, b"");
    }
}

/// `%SystemRoot%\System32\MdSched.exe`, which asks for elevation itself.
pub fn memory_diagnostic_exe() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    PathBuf::from(root).join("System32").join("MdSched.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_args_quote_the_output() {
        let out = Path::new(r"C:\Users\some one\AppData\Local\Vestigium\data\tests\5.surface");
        assert_eq!(
            disk_scan_args(1, 0, out),
            r#"disk-scan --disk 1 --out "C:\Users\some one\AppData\Local\Vestigium\data\tests\5.surface""#
        );
        assert!(disk_scan_args(1, 4096, out).ends_with(" --from 4096"));
        let out = Path::new(r"C:\data\tests\6.chkdsk");
        assert_eq!(
            volume_check_args('D', out),
            r#"volume-check --volume D: --out "C:\data\tests\6.chkdsk""#
        );
    }

    #[test]
    fn tests_file_sits_beside_the_database() {
        let db = Path::new(r"C:\Users\someone\AppData\Local\Vestigium\data\vestigium.db");
        assert_eq!(
            tests_file(db, 7, "test").unwrap(),
            PathBuf::from(r"C:\Users\someone\AppData\Local\Vestigium\data\tests\7.test")
        );
    }

    #[test]
    fn letters_come_only_from_drive_roots() {
        assert_eq!(letter_of(r"d:\"), Some('D'));
        assert_eq!(letter_of("C:"), Some('C'));
        assert_eq!(letter_of(r"\\server\share"), None);
        assert_eq!(letter_of("1:"), None);
    }

    fn write_progress(dir: &Path, stem: &str, ts_ms: i64) {
        let progress = SurfaceProgress {
            disk: 1,
            ts_ms,
            at_bytes: 5,
            from_bytes: 0,
            disk_bytes: 10,
            bands: [0; vestigium_core::hwtest::BANDS],
            unreadable_blocks: 0,
        };
        let mut bytes = vec![TEST_RUN_VERSION];
        bytes.extend(postcard::to_allocvec(&progress).unwrap());
        std::fs::write(dir.join(format!("{stem}.progress")), &bytes).unwrap();
    }

    #[test]
    fn an_empty_surface_file_beside_fresh_progress_is_a_scan_under_way() {
        let dir = tempfile::tempdir().unwrap();
        write_progress(dir.path(), "7", 10_000);
        std::fs::write(dir.path().join("7.surface"), []).unwrap();
        let found = scan_under_way(dir.path(), 20_000).unwrap();
        assert_eq!(found.stem, "7");
        assert!(scan_under_way(dir.path(), 200_000).is_none(), "stale");
        std::fs::write(dir.path().join("7.surface"), [1, 2, 3]).unwrap();
        assert!(scan_under_way(dir.path(), 20_000).is_none(), "finished");
    }

    #[test]
    fn the_stop_file_sits_beside_the_scan_result() {
        let dir = Path::new(r"C:\data	ests");
        let result = dir.join("9.surface");
        assert_eq!(stop_file_of(dir, "9"), result.with_extension("stop"));
    }

    #[test]
    fn a_fresh_progress_file_without_a_result_is_a_scan_under_way() {
        let dir = tempfile::tempdir().unwrap();
        let progress = SurfaceProgress {
            disk: 1,
            ts_ms: 10_000,
            at_bytes: 5,
            from_bytes: 0,
            disk_bytes: 10,
            bands: [0; vestigium_core::hwtest::BANDS],
            unreadable_blocks: 0,
        };
        let mut bytes = vec![TEST_RUN_VERSION];
        bytes.extend(postcard::to_allocvec(&progress).unwrap());
        std::fs::write(dir.path().join("42.progress"), &bytes).unwrap();
        let found = scan_under_way(dir.path(), 20_000).unwrap();
        assert_eq!(found.stem, "42");
        assert!(scan_under_way(dir.path(), 200_000).is_none(), "stale");
        std::fs::write(dir.path().join("42.surface"), [1]).unwrap();
        assert!(scan_under_way(dir.path(), 20_000).is_none(), "finished");
    }
}
