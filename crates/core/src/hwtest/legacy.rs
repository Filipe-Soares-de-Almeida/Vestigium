use serde::{Deserialize, Serialize};

use super::{
    BANDS, Benchmark, MemoryQuick, Outcome, Region, SlowBlock, SurfaceScan, TestData, TestRun,
    VolumeCheck, VolumeResult,
};
use crate::Media;
use crate::health::Subject;

/// A `TestRun` as version 2 of its layout stored it, before a surface scan kept whether other
/// programs used the disk. The variants keep their order: postcard writes an enum by its position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestRunV2 {
    pub subject: Subject,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub outcome: Outcome,
    pub data: TestDataV2,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TestDataV2 {
    MemoryQuick(MemoryQuick),
    DiskSurface(SurfaceScanV2),
    VolumeCheck(VolumeCheck),
    DiskBenchmark(Benchmark),
}

/// A `SurfaceScan` without the outside traffic flag.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceScanV2 {
    pub disk: u32,
    pub media: Media,
    pub disk_bytes: u64,
    pub sector_bytes: u32,
    pub from_bytes: u64,
    pub to_bytes: u64,
    pub bands: [u64; BANDS],
    pub unreadable_blocks: u64,
    pub regions: Vec<Region>,
    pub worst: Vec<SlowBlock>,
    pub unreadable: Vec<u64>,
    pub failure: Option<String>,
}

impl From<SurfaceScanV2> for SurfaceScan {
    fn from(old: SurfaceScanV2) -> SurfaceScan {
        SurfaceScan {
            disk: old.disk,
            media: old.media,
            disk_bytes: old.disk_bytes,
            sector_bytes: old.sector_bytes,
            from_bytes: old.from_bytes,
            to_bytes: old.to_bytes,
            bands: old.bands,
            unreadable_blocks: old.unreadable_blocks,
            regions: old.regions,
            worst: old.worst,
            unreadable: old.unreadable,
            failure: old.failure,
            outside_io: false,
        }
    }
}

impl From<TestRunV2> for TestRun {
    fn from(old: TestRunV2) -> TestRun {
        TestRun {
            subject: old.subject,
            started_ms: old.started_ms,
            ended_ms: old.ended_ms,
            outcome: old.outcome,
            data: match old.data {
                TestDataV2::MemoryQuick(d) => TestData::MemoryQuick(d),
                TestDataV2::DiskSurface(d) => TestData::DiskSurface(d.into()),
                TestDataV2::VolumeCheck(d) => TestData::VolumeCheck(d),
                TestDataV2::DiskBenchmark(d) => TestData::DiskBenchmark(d),
            },
        }
    }
}

/// A `TestRun` as version 1 of its layout stored it, before a volume check kept the volume's dirty
/// state. The variants keep their order: postcard writes an enum by its position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestRunV1 {
    pub subject: Subject,
    pub started_ms: i64,
    pub ended_ms: i64,
    pub outcome: Outcome,
    pub data: TestDataV1,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TestDataV1 {
    MemoryQuick(MemoryQuick),
    DiskSurface(SurfaceScanV2),
    VolumeCheck(VolumeCheckV1),
    DiskBenchmark(Benchmark),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumeCheckV1 {
    pub volume: String,
    pub exit_code: Option<u32>,
    pub result: VolumeResult,
    pub output: String,
}

impl From<TestRunV1> for TestRun {
    fn from(old: TestRunV1) -> TestRun {
        TestRun {
            subject: old.subject,
            started_ms: old.started_ms,
            ended_ms: old.ended_ms,
            outcome: old.outcome,
            data: match old.data {
                TestDataV1::MemoryQuick(d) => TestData::MemoryQuick(d),
                TestDataV1::DiskSurface(d) => TestData::DiskSurface(d.into()),
                TestDataV1::VolumeCheck(d) => TestData::VolumeCheck(VolumeCheck {
                    volume: d.volume,
                    exit_code: d.exit_code,
                    result: d.result,
                    output: d.output,
                    dirty_before: None,
                    dirty_after: None,
                }),
                TestDataV1::DiskBenchmark(d) => TestData::DiskBenchmark(d),
            },
        }
    }
}
