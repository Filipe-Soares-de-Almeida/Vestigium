mod reliability;
mod rules;
mod smart;

use serde::{Deserialize, Serialize};

use crate::{EventMeaning, MachineInfo, Media, NextStep, WinEvent};

pub use reliability::{failures_between, is_failure};
pub use rules::evaluate;
pub use smart::{
    NVME_WORN_PCT, NvmeHealth, SmartAttribute, WATCHED_ATTRIBUTES, nvme_health, smart_attributes,
};

/// Layout of the `Finding` BLOB written today.
pub const FINDING_VERSION: u8 = 1;

/// How much a finding asks for attention; findings below attention are not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Concern {
    Attention,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FindingKind {
    DiskSmart,
    LowSpace,
    PageFile,
    MemoryOften,
    DisplayResets,
    HardwareErrors,
    StorageErrors,
    Throttling,
    WindowsOnHdd,
    TwoAntivirus,
    SlowBoot,
    DirtyRestarts,
    ShutdownUnfinished,
    FailuresRising,
    NoCrashDump,
}

/// What a finding is about: one disk or volume, or the whole PC.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Subject {
    Pc,
    /// A physical disk, by its instance name such as "0 C:".
    Disk(String),
    /// A volume, by its root such as "C:\".
    Volume(String),
}

/// A value as the findings show it; the UI words it in the chosen language.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Measure {
    Count(u32),
    Percent(f32),
    Days(u32),
    Seconds(f32),
    Bytes(u64),
    Media(Media),
}

/// The range a healthy PC stays in, shown next to the value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Reference {
    AtMost(Measure),
    AtLeast(Measure),
    Is(Measure),
}

/// A product the Windows Security Center knows, and whether it is on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityProduct {
    pub name: String,
    pub on: bool,
}

/// A step of the boot that Windows measured as slow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelayKind {
    App,
    Driver,
    Service,
    Phase,
    Device,
}

/// One boot Windows measured, from Diagnostics-Performance event 100.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BootTime {
    pub ts_ms: i64,
    pub boot_ms: u32,
    pub main_path_ms: u32,
    pub post_boot_ms: u32,
}

/// What slowed a boot, from Diagnostics-Performance events 101 to 110.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BootDelay {
    pub ts_ms: i64,
    pub kind: DelayKind,
    pub name: String,
    pub total_ms: u32,
    pub degradation_ms: u32,
    /// A security product, which the findings never suggest turning off.
    pub security: bool,
}

/// What the last full scan read from the boot diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BootReport {
    pub boots: Vec<BootTime>,
    pub delays: Vec<BootDelay>,
}

/// Layout of the `ScanResult` the elevated helper writes and the store keeps.
pub const SCAN_VERSION: u8 = 1;

/// What one full scan read with elevation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScanResult {
    pub ts_ms: i64,
    pub boot: BootReport,
}

/// One physical disk's health, read without elevation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiskHealth {
    pub name: String,
    pub read_ms: i64,
    /// The disk's own verdict against its attribute thresholds.
    pub predict_failure: bool,
    pub attributes: Vec<SmartAttribute>,
    pub nvme: Option<NvmeHealth>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeSpace {
    pub root: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
    /// Holds the Windows folder.
    pub system: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageFile {
    pub path: String,
    pub size_bytes: u64,
    pub peak_bytes: u64,
}

/// Everything one health pass looks at. `None` means the source was not read, not that it is empty.
#[derive(Clone, Debug, PartialEq)]
pub struct HealthInputs {
    pub now_ms: i64,
    /// Oldest time the event history covers; trends need five weeks of it.
    pub history_from_ms: i64,
    pub machine: MachineInfo,
    /// Instance name of the physical disk that holds Windows.
    pub system_disk: Option<String>,
    pub disks: Vec<DiskHealth>,
    pub volumes: Vec<VolumeSpace>,
    pub page_files: Option<Vec<PageFile>>,
    /// Highest commit charge of the last 7 days, as a share of the commit limit.
    pub commit_peak_pct_7d: Option<f32>,
    /// Days of the last 7 with a high-memory episode.
    pub memory_high_days_7d: u32,
    /// Curated events of the last 35 days or more.
    pub events: Vec<WinEvent>,
    pub antivirus: Option<Vec<SecurityProduct>>,
    pub boot: Option<BootReport>,
    /// Findings already open, which stay open until their value is clearly back in range.
    pub open: Vec<(FindingKind, Subject)>,
}

impl HealthInputs {
    fn is_open(&self, kind: FindingKind, subject: &Subject) -> bool {
        self.open.iter().any(|(k, s)| *k == kind && s == subject)
    }

    fn events_since(&self, days: i64) -> impl Iterator<Item = &WinEvent> {
        let from = self.now_ms - days * DAY_MS;
        self.events.iter().filter(move |e| e.ts_ms >= from)
    }
}

/// A fact behind a finding; the UI words it in the chosen language.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HealthFact {
    Attribute(SmartAttribute),
    PredictFailure,
    Nvme(NvmeHealth),
    Event {
        ts_ms: i64,
        provider: String,
        id: u16,
        meaning: EventMeaning,
    },
    Space {
        free_bytes: u64,
        total_bytes: u64,
    },
    PageFile(PageFile),
    NoPageFile,
    CommitPeak {
        pct: f32,
    },
    MemoryHighDays {
        days: u32,
    },
    Product(SecurityProduct),
    Boot(BootTime),
    Delay(BootDelay),
    Failures {
        last_week: u32,
        weekly_before: f32,
    },
    SystemDisk {
        disk: String,
        media: Media,
    },
}

/// A risk found on the PC, with its value, the healthy range and the facts behind it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub subject: Subject,
    pub concern: Concern,
    pub value: Measure,
    pub reference: Reference,
    pub evidence: Vec<HealthFact>,
    pub next_steps: Vec<NextStep>,
}

/// A finding as stored, with its row and when it opened.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredFinding {
    pub id: i64,
    pub opened_ms: i64,
    pub finding: Finding,
}

/// What a health pass changes in the stored findings.
#[derive(Clone, Debug, PartialEq)]
pub enum FindingChange {
    Open(Finding),
    Update { id: i64, finding: Finding },
    Close { id: i64 },
}

/// Turns a pass's findings into changes: new ones open, changed ones update, missing ones close.
pub fn reconcile(open: &[StoredFinding], fresh: &[Finding]) -> Vec<FindingChange> {
    let mut changes = Vec::new();
    for finding in fresh {
        match open
            .iter()
            .find(|s| s.finding.kind == finding.kind && s.finding.subject == finding.subject)
        {
            Some(stored) if stored.finding != *finding => changes.push(FindingChange::Update {
                id: stored.id,
                finding: finding.clone(),
            }),
            Some(_) => {}
            None => changes.push(FindingChange::Open(finding.clone())),
        }
    }
    for stored in open {
        let still = fresh
            .iter()
            .any(|f| f.kind == stored.finding.kind && f.subject == stored.finding.subject);
        if !still {
            changes.push(FindingChange::Close { id: stored.id });
        }
    }
    changes
}

pub(crate) const DAY_MS: i64 = 24 * 60 * 60_000;
