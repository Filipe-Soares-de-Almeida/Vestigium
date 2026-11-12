mod disk;
mod legacy;
mod memory;

use serde::{Deserialize, Serialize};

use crate::health::Subject;
use crate::{Episode, EpisodeKind};

pub use disk::{
    BAND_BOUNDS_MS, BANDS, BENCH_FILE_BYTES, BENCH_FREE_MARGIN_BYTES, BENCH_SECONDS, BenchMode,
    BenchResult, Benchmark, Bus, Expected, Region, SLOW_BLOCK_MS, SURFACE_BLOCK, SURFACE_REGIONS,
    SlowBlock, SurfaceAggregator, SurfaceProgress, SurfaceScan, UNREADABLE_KEPT, VolumeCheck,
    VolumeResult, WORST_KEPT, bench_has_room, bench_reference, bench_verdict, combined_surface,
    latency_band, outside_traffic, volume_result,
};
pub use legacy::{SurfaceScanV2, TestDataV1, TestDataV2, TestRunV1, TestRunV2, VolumeCheckV1};
pub use memory::{
    DiagnosticDetail, DiagnosticResult, ERRORS_KEPT, MemoryDiagnostic, MemoryError, MemoryQuick,
    PATTERNS, Pattern, QUICK_CHUNK, QUICK_LEAVE_FREE, QUICK_MIN, QUICK_MINUTES, QUICK_USE_CAP,
    memory_diagnostics, quick_test_size,
};

/// Layout of the `TestRun` the window and the helper write and the store keeps; versions 1 and 2
/// are still read through `TestRunV1` and `TestRunV2`.
pub const TEST_RUN_VERSION: u8 = 3;
/// How far back the health pass looks at test runs.
pub const TEST_REACH_DAYS: i64 = 90;
/// The Vestigium processes that run tests: the window and the elevated helper.
pub const TEST_PROCESSES: [&str; 2] = ["Vestigium.Window.exe", "Vestigium.Helper.exe"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestKind {
    MemoryQuick,
    DiskSurface,
    VolumeCheck,
    DiskBenchmark,
}

impl TestKind {
    /// Every variant, in declaration order.
    pub const ALL: [TestKind; 4] = [
        TestKind::MemoryQuick,
        TestKind::DiskSurface,
        TestKind::VolumeCheck,
        TestKind::DiskBenchmark,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Ran to the end and found nothing.
    Clean,
    /// Found what it looks for: memory errors, unreadable sectors, file system problems.
    Problems,
    /// Stopped before the end; what it covered still counts.
    Interrupted,
    /// Could not run; the reason is in the data.
    Failed,
}

impl Outcome {
    /// Every variant, in declaration order.
    pub const ALL: [Outcome; 4] = [
        Outcome::Clean,
        Outcome::Problems,
        Outcome::Interrupted,
        Outcome::Failed,
    ];
}

/// What a test covered and found, in the shape of its kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TestData {
    MemoryQuick(MemoryQuick),
    DiskSurface(SurfaceScan),
    VolumeCheck(VolumeCheck),
    DiskBenchmark(Benchmark),
}

/// One run of a test, as the window or the helper reports it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestRun {
    pub subject: Subject,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub outcome: Outcome,
    pub data: TestData,
}

impl TestRun {
    pub fn kind(&self) -> TestKind {
        match self.data {
            TestData::MemoryQuick(_) => TestKind::MemoryQuick,
            TestData::DiskSurface(_) => TestKind::DiskSurface,
            TestData::VolumeCheck(_) => TestKind::VolumeCheck,
            TestData::DiskBenchmark(_) => TestKind::DiskBenchmark,
        }
    }
}

/// The disk subject for physical drive `number`, named as the disk counters name it when known.
pub fn disk_subject(number: u32, disks: &[crate::DiskInfo]) -> Subject {
    let name = disks
        .iter()
        .find(|d| d.name.split_whitespace().next() == Some(number.to_string().as_str()))
        .map_or_else(|| number.to_string(), |d| d.name.clone());
    Subject::Disk(name)
}

/// Whether an episode most likely came from a Vestigium test running at the time: the process
/// is one of ours, or a busy disk is the one a disk test was reading or writing.
pub fn caused_by_test(episode: &Episode, runs: &[TestRun]) -> bool {
    let ours = episode.culprit.as_ref().is_some_and(|c| {
        TEST_PROCESSES
            .iter()
            .any(|p| p.eq_ignore_ascii_case(&c.name))
    });
    let overlapping = || {
        runs.iter()
            .filter(|r| r.started_ms <= episode.end_ms && episode.start_ms <= r.ended_ms)
    };
    if ours && overlapping().next().is_some() {
        return true;
    }
    let EpisodeKind::DiskBusy { disk } = &episode.kind else {
        return false;
    };
    overlapping().any(|r| {
        matches!(
            r.kind(),
            TestKind::DiskSurface | TestKind::VolumeCheck | TestKind::DiskBenchmark
        ) && tested_disk(&r.subject, disk)
    })
}

/// Whether a test of `subject` works on the disk the counters call `disk`, such as "1 D:": a disk
/// by its name or number, a volume by its drive letter.
fn tested_disk(subject: &Subject, disk: &str) -> bool {
    let mut parts = disk.split_whitespace();
    let number = parts.next();
    match subject {
        Subject::Disk(name) => {
            name.eq_ignore_ascii_case(disk) || name.split_whitespace().next() == number
        }
        Subject::Volume(root) => root
            .get(..2)
            .is_some_and(|letter| parts.any(|v| v.eq_ignore_ascii_case(letter))),
        Subject::Pc => false,
    }
}
