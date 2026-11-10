use serde::{Deserialize, Serialize};

use super::{
    Benchmark, MemoryQuick, Outcome, SurfaceScan, TestData, TestRun, VolumeCheck, VolumeResult,
};
use crate::health::Subject;

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
    DiskSurface(SurfaceScan),
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
                TestDataV1::DiskSurface(d) => TestData::DiskSurface(d),
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
