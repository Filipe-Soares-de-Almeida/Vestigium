//! Serializes a sample of every type the window receives into a TypeScript file that checks each
//! sample against `lib/types.ts`, so a Rust change the types do not follow fails `npm run check`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde::Serialize;
use vestigium_core::health::{
    BootDelay, BootReport, BootTime, Concern, DelayKind, DiskHealth, Finding, FindingKind,
    HealthFact, Measure, NvmeDetail, NvmeHealth, PageFile, Reference, SecurityProduct,
    SmartAttribute, Subject,
};
use vestigium_core::hwtest::{
    BANDS, BenchMode, BenchResult, Benchmark, Bus, DiagnosticDetail, DiagnosticResult,
    MemoryDiagnostic, MemoryError, MemoryQuick, Outcome, PATTERNS, Region, SlowBlock,
    SurfaceProgress, SurfaceScan, TestData, TestKind, TestRun, VolumeCheck, VolumeResult,
};
use vestigium_core::{
    Advice, AppFrames, Applier, BUCKETS, CaptureSource, Cause, Channel, ColorTheme, Confidence,
    CoreLoad, CrashKind, Culprit, DeepCapture, DetailLevel, DiskInfo, DiskLoad, DriverLatency,
    DumpCulprit, DumpKind, Episode, EpisodeKind, EventMeaning, Evidence, Fact, FactKind,
    FrameHitch, Gap, GapReason, Guid, HangOutcome, HangSpan, Helped, Hive, HungWindow,
    LanguagePref, LongRoutine, MachineInfo, Media, Metric, NextStep, Occurrence, OccurrenceKind,
    OccurrenceSummary, PcState, ProbeHiccup, ProcessSample, ProcessTop, ProgramPoint, Reading,
    RestorePoint, Retention, RoutineKind, RoutineStats, RunAs, Sample, Setting, SettingChange,
    SettingResult, SettingValue, Settings, Severity, Signal, Signature, StartupList, Status,
    Suspect, TWEAKS, Takes, ThemePref, Trigger, TweakChange, Value, WinEvent, Window,
};
use vestigium_ipc::{AgentStatus, IpcError, NowFrame, TrayState};
use vestigium_store::{CaptureSummary, StoreError};

use crate::about::AboutDto;
use crate::agent::{AgentLink, SettingsDto, UiError};
use crate::capture::{CaptureFinished, CaptureRun, DeleteFinished, DumpScanFinished, ScanFinished};
use crate::history::{
    BugCheckDto, CaptureDto, CaptureNote, Columns, CrashControlDto, DumpDto, DumpsDto,
    EpisodeDetailDto, FaultDto, FindingDto, HealthDto, HistoryState, MarksDto, OccurrenceDto,
    ProgramMarksDto, Resolution, SmartPointDto, TestSpan, TimelineDto,
};
use crate::hwtests::{
    BenchRange, DiskDetailDto, DiskDto, MemoryDto, RunningTest, ScanUnderWay, TestFinished,
    TestProgress, TestRunDto, TestsDto, VolumeDto,
};
use crate::live::{Lanes, LiveMessage, LivePoint};
use crate::process::ProcessFactsDto;
use crate::tweaks::{TweakRowDto, TweaksApplied, TweaksDto};
use crate::update::{CheckResult, Group, Progress, Release, UpdatePrefs};

const FIXTURE: &str = "../src/dev/fixtures/wire.ts";
const HEADER: &str = "// Written by the `wire` test of the window crate; do not edit by hand.\n\
// Refresh with `UPDATE_FIXTURES=1 cargo test -p vestigium-ui wire`.\n";
const TYPES: &str = "../../lib/types";
const UPDATE: &str = "../../lib/update.svelte";
const ABOUT: &str = "../../screens/settings/data";
/// Types of `lib/types.ts` the window sends but never receives, so no sample exists for them.
const SENT_ONLY: [&str; 1] = ["TweakPick"];

/// Lists one sample per variant; the `match` stops compiling when a variant of the enum is
/// missing from the list, and each sample must be the variant it is listed under.
macro_rules! variants {
    ($ty:ident: $($variant:ident => $sample:expr),+ $(,)?) => {{
        let _exhaustive = |value: &$ty| match value {
            $($ty::$variant { .. } => ()),+
        };
        vec![$({
            let sample = $sample;
            assert!(
                matches!(sample, $ty::$variant { .. }),
                concat!(stringify!($ty), "::", stringify!($variant), " sample is another variant"),
            );
            sample
        }),+]
    }};
}

/// The samples of every type, rendered as one TypeScript module.
#[derive(Default)]
struct Wire {
    imports: BTreeMap<&'static str, BTreeSet<&'static str>>,
    body: String,
}

impl Wire {
    fn add<T: Serialize>(&mut self, from: &'static str, ts_type: &'static str, samples: Vec<T>) {
        assert!(!samples.is_empty(), "no sample for {ts_type}");
        self.imports.entry(from).or_default().insert(ts_type);
        let name = format!("{}{}Samples", ts_type[..1].to_lowercase(), &ts_type[1..]);
        self.body.push_str(&format!("\nexport const {name} = [\n"));
        for sample in &samples {
            let json = serde_json::to_string(sample).unwrap();
            self.body.push_str(&format!("  {json},\n"));
        }
        self.body.push_str(&format!("] satisfies {ts_type}[];\n"));
    }

    fn text(&self) -> String {
        let mut out = String::from(HEADER);
        for (from, names) in &self.imports {
            let names: Vec<&str> = names.iter().copied().collect();
            out.push_str(&format!(
                "import type {{ {} }} from \"{from}\";\n",
                names.join(", ")
            ));
        }
        out.push_str(&self.body);
        out
    }
}

fn first<T>(mut all: Vec<T>, count: usize) -> Vec<T> {
    all.truncate(count);
    all
}

fn string(text: &str) -> String {
    text.to_owned()
}

/// A sample with every optional field filled and one with every one empty.
fn both<T>(make: impl Fn(bool) -> T) -> Vec<T> {
    vec![make(true), make(false)]
}

fn some(full: bool, text: &str) -> Option<String> {
    full.then(|| string(text))
}

fn guid(low: u128) -> Guid {
    Guid(0x2a73_7441_1930_4402_8d77_b2be_bba3_0000 + low)
}

fn reading(full: bool) -> Reading {
    Reading {
        ts_ms: 1_790_000_000_000,
        cpu_pct: 12.5,
        mem_used_pct: full.then_some(61.5),
        disk_active_pct: 7.5,
        disk_latency_ms: 1.5,
        gpu_pct: full.then_some(30.0),
        probe_lag_ms: 0.5,
    }
}

fn sample(full: bool) -> Sample {
    Sample {
        ts_ms: 1_790_000_000_000,
        cpu_pct: 12.5,
        dpc_pct: 0.5,
        isr_pct: 0.25,
        perf_limit_pct: full.then_some(100.0),
        proc_perf_pct: full.then_some(90.0),
        mem_total_bytes: 17_179_869_184,
        mem_avail_bytes: 8_589_934_592,
        commit_bytes: 9_000_000_000,
        commit_limit_bytes: 20_000_000_000,
        hard_faults_per_s: full.then_some(3.5),
        gpu_pct: full.then_some(30.0),
        probe_max_lag_ms: 4.5,
        sampler_lag_ms: 0.5,
        cores: vec![CoreLoad {
            cpu_pct: 10.0,
            dpc_pct: 0.5,
            isr_pct: 0.25,
        }],
        disks: vec![DiskLoad {
            name: string("0 C:"),
            active_pct: 7.5,
            latency_ms: full.then_some(1.5),
            queue_len: full.then_some(0.5),
            bytes_per_s: full.then_some(1024.0),
        }],
    }
}

fn process_sample() -> ProcessSample {
    ProcessSample {
        pid: 4120,
        name: string("example.exe"),
        cpu_pct: 14.5,
        private_bytes: 123_456_789,
        read_bps: 2048.0,
        write_bps: 1024.0,
        gpu_pct: 3.5,
    }
}

fn top() -> ProcessTop {
    ProcessTop {
        ts_ms: 1_790_000_000_000,
        entries: vec![process_sample()],
    }
}

fn win_event(channel: Channel, meaning: EventMeaning) -> WinEvent {
    WinEvent {
        ts_ms: 1_790_000_000_000,
        channel,
        provider: string("example-provider"),
        id: 129,
        record_id: 99_001,
        level: 2,
        meaning,
        data_xml: string("<Data Name=\"Example\">1</Data>"),
    }
}

fn gap(reason: GapReason) -> Gap {
    Gap {
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_060_000,
        reason,
    }
}

fn hang_span(outcome: HangOutcome) -> HangSpan {
    HangSpan {
        pid: 4120,
        name: string("example.exe"),
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_030_000,
        outcome,
    }
}

fn machine() -> MachineInfo {
    MachineInfo {
        logical_cpus: 8,
        mem_total_bytes: 17_179_869_184,
        disks: Media::ALL
            .iter()
            .map(|&media| DiskInfo {
                name: string("0 C:"),
                media,
            })
            .collect(),
    }
}

fn window(full: bool) -> Window {
    Window {
        from_ms: 1_789_999_900_000,
        to_ms: 1_790_000_100_000,
        samples: vec![sample(full)],
        tops: vec![top()],
        events: Channel::ALL
            .iter()
            .map(|&channel| win_event(channel, EventMeaning::StorageReset))
            .collect(),
        hangs: HangOutcome::ALL.iter().map(|&o| hang_span(o)).collect(),
        gaps: GapReason::ALL.iter().map(|&r| gap(r)).collect(),
        machine: machine(),
    }
}

fn causes() -> Vec<Cause> {
    let mut all = variants!(Cause:
        Storage => Cause::Storage { disk: Some(string("0 C:")) },
        DiskIo => Cause::DiskIo { disk: string("0 C:"), hdd: true, process: Some(string("example.exe")) },
        Memory => Cause::Memory { process: Some(string("example.exe")) },
        Cpu => Cause::Cpu { processes: vec![string("example.exe")] },
        Driver => Cause::Driver { core: 3 },
        GpuDriver => Cause::GpuDriver,
        Throttling => Cause::Throttling,
        App => Cause::App { name: string("example.exe") },
        Unclear => Cause::Unclear,
        DriverModule => Cause::DriverModule { module: string("example.sys"), intermediary: false },
        Frames => Cause::Frames { app: string("example.exe") },
        BlueScreen => Cause::BlueScreen { code: 0x9f },
        Unresponsive => Cause::Unresponsive,
        SleepTransition => Cause::SleepTransition,
        PowerCut => Cause::PowerCut { no_dump: true },
        HardwareFault => Cause::HardwareFault,
        NotMeasured => Cause::NotMeasured,
    );
    all.extend([
        Cause::Storage { disk: None },
        Cause::DiskIo {
            disk: string("1 D:"),
            hdd: false,
            process: None,
        },
        Cause::Memory { process: None },
        Cause::DriverModule {
            module: string("example.sys"),
            intermediary: true,
        },
        Cause::PowerCut { no_dump: false },
    ]);
    all
}

fn next_steps() -> Vec<NextStep> {
    variants!(NextStep:
        UpdateStorageDriver => NextStep::UpdateStorageDriver,
        CheckDiskCable => NextStep::CheckDiskCable,
        RunDiskTest => NextStep::RunDiskTest,
        FindDiskHeavyApp => NextStep::FindDiskHeavyApp,
        CloseMemoryHeavyApp => NextStep::CloseMemoryHeavyApp,
        CheckCooling => NextStep::CheckCooling,
        CheckPowerPlan => NextStep::CheckPowerPlan,
        UpdateGpuDriver => NextStep::UpdateGpuDriver,
        EnableSensitiveMode => NextStep::EnableSensitiveMode,
        DeepCapture => NextStep::DeepCapture,
        ReportToAppVendor => NextStep::ReportToAppVendor,
        UpdateDriver => NextStep::UpdateDriver,
        BackUpNow => NextStep::BackUpNow,
        ReplaceDisk => NextStep::ReplaceDisk,
        FreeDiskSpace => NextStep::FreeDiskSpace,
        LetWindowsManagePageFile => NextStep::LetWindowsManagePageFile,
        CheckHardwareStability => NextStep::CheckHardwareStability,
        KeepOneAntivirus => NextStep::KeepOneAntivirus,
        ReviewStartupApps => NextStep::ReviewStartupApps,
        WaitForShutdown => NextStep::WaitForShutdown,
        MoveWindowsToSsd => NextStep::MoveWindowsToSsd,
        RunMemoryDiagnostic => NextStep::RunMemoryDiagnostic,
        RunMemTest86 => NextStep::RunMemTest86,
        RepairVolume => NextStep::RepairVolume,
        ReadCrashDump => NextStep::ReadCrashDump,
    )
}

fn crash_kinds() -> Vec<CrashKind> {
    variants!(CrashKind:
        BlueScreen => CrashKind::BlueScreen { code: 0x50, params: [1, 2, 3, 4] },
        PowerButton => CrashKind::PowerButton,
        WhileShuttingDown => CrashKind::WhileShuttingDown,
        WhileSleeping => CrashKind::WhileSleeping,
        NoTrace => CrashKind::NoTrace,
    )
}

fn routine_kinds() -> Vec<RoutineKind> {
    variants!(RoutineKind: Dpc => RoutineKind::Dpc, Isr => RoutineKind::Isr)
}

fn evidence() -> Vec<Evidence> {
    let mut all = variants!(Evidence:
        Event => Evidence::Event { ts_ms: 1_790_000_000_000, provider: string("example-provider"), id: 129, meaning: EventMeaning::StorageReset },
        Metric => Evidence::Metric { ts_ms: 1_790_000_000_000, metric: Metric::DiskLatencyMs, value: 450.5, threshold: 100.0, subject: Some(string("0 C:")) },
        Process => Evidence::Process { pid: 4120, name: string("example.exe"), metric: Metric::PrivateBytes, value: 1.5e9 },
        Hang => Evidence::Hang { name: string("example.exe"), duration_ms: 31_000 },
        Routine => Evidence::Routine { module: string("example.sys"), kind: RoutineKind::Dpc, count: 120, max_us: 900, at_least_100us: 40, at_least_1ms: 2, core_pct: 4.5 },
        LongRoutine => Evidence::LongRoutine { ts_ms: 1_790_000_000_000, module: string("example.sys"), kind: RoutineKind::Isr, us: 1500 },
        Frames => Evidence::Frames { app: string("example.exe"), frames: 600, hitches: 12, worst_ms: 80.5, median_ms: 8.5 },
        Hitch => Evidence::Hitch { ts_ms: 1_790_000_000_000, app: string("example.exe"), frame_ms: 70.5 },
        Hiccup => Evidence::Hiccup { ts_ms: 1_790_000_000_000, lag_ms: 250.5 },
        CaptureLoss => Evidence::CaptureLoss { lost_events: 17 },
    );
    all.push(Evidence::Metric {
        ts_ms: 1_790_000_000_000,
        metric: Metric::CpuPct,
        value: 99.5,
        threshold: 90.0,
        subject: None,
    });
    all.extend(
        routine_kinds()
            .into_iter()
            .map(|kind| Evidence::LongRoutine {
                ts_ms: 1_790_000_000_000,
                module: string("example.sys"),
                kind,
                us: 800,
            }),
    );
    all
}

fn suspects() -> Vec<Suspect> {
    let mut all: Vec<Suspect> = causes()
        .into_iter()
        .zip(Confidence::ALL.iter().cycle())
        .map(|(cause, &confidence)| Suspect {
            cause,
            confidence,
            evidence: Vec::new(),
            next_steps: Vec::new(),
        })
        .collect();
    all.push(Suspect {
        cause: Cause::Unclear,
        confidence: Confidence::Medium,
        evidence: evidence(),
        next_steps: next_steps(),
    });
    all
}

/// One suspect with a little evidence, for the types that nest suspects.
fn small_suspects() -> Vec<Suspect> {
    vec![Suspect {
        cause: Cause::Unclear,
        confidence: Confidence::Medium,
        evidence: first(evidence(), 2),
        next_steps: first(next_steps(), 2),
    }]
}

fn triggers() -> Vec<Trigger> {
    let mut all = variants!(Trigger:
        Freeze => Trigger::Freeze { start_ms: 1_790_000_000_000, end_ms: 1_790_000_005_000, lag_ms: 5000.5 },
        Stutter => Trigger::Stutter { start_ms: 1_790_000_000_000, end_ms: 1_790_000_004_000, count: 6, worst_ms: 220.5 },
        AppHang => Trigger::AppHang { pid: 4120, name: string("example.exe"), start_ms: 1_790_000_000_000, end_ms: 1_790_000_030_000, outcome: HangOutcome::Recovered },
        Reported => Trigger::Reported { event: win_event(Channel::Application, EventMeaning::AppCrash) },
        Crash => Trigger::Crash { last_ms: 1_790_000_000_000, down_ms: 1_790_000_300_000, kind: CrashKind::NoTrace },
    );
    all.extend(HangOutcome::ALL.iter().map(|&outcome| Trigger::AppHang {
        pid: 4120,
        name: string("example.exe"),
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_030_000,
        outcome,
    }));
    all.extend(crash_kinds().into_iter().map(|kind| Trigger::Crash {
        last_ms: 1_790_000_000_000,
        down_ms: 1_790_000_300_000,
        kind,
    }));
    all
}

fn fact_kinds() -> Vec<FactKind> {
    let mut all = variants!(FactKind:
        FreezeStarted => FactKind::FreezeStarted { duration_ms: 5000.5 },
        FreezeEnded => FactKind::FreezeEnded,
        Stutter => FactKind::Stutter { count: 6, worst_ms: 220.5 },
        AppHung => FactKind::AppHung { name: string("example.exe") },
        AppRecovered => FactKind::AppRecovered { name: string("example.exe") },
        AppClosedWhileHung => FactKind::AppClosedWhileHung { name: string("example.exe") },
        Event => FactKind::Event { meaning: EventMeaning::DiskRetry, provider: string("example-provider"), id: 153 },
        Crossed => FactKind::Crossed { metric: Metric::MemAvailPct, value: 4.5, threshold: 10.0, subject: Some(string("0 C:")) },
        Busiest => FactKind::Busiest { metric: Metric::CpuPct, name: string("example.exe"), value: 88.5 },
        Gap => FactKind::Gap { reason: GapReason::ClockChanged },
        LastWatched => FactKind::LastWatched,
        WentDown => FactKind::WentDown { kind: CrashKind::PowerButton },
        EventsDropped => FactKind::EventsDropped { count: 40 },
        AppStillHung => FactKind::AppStillHung { name: string("example.exe") },
    );
    all.push(FactKind::Crossed {
        metric: Metric::CpuPct,
        value: 99.5,
        threshold: 90.0,
        subject: None,
    });
    all.extend(
        GapReason::ALL
            .iter()
            .map(|&reason| FactKind::Gap { reason }),
    );
    all.extend(
        crash_kinds()
            .into_iter()
            .map(|kind| FactKind::WentDown { kind }),
    );
    all.extend(Metric::ALL.iter().map(|&metric| FactKind::Busiest {
        metric,
        name: string("example.exe"),
        value: 50.5,
    }));
    all.extend(EventMeaning::ALL.iter().map(|&meaning| FactKind::Event {
        meaning,
        provider: string("example-provider"),
        id: 1,
    }));
    all
}

fn episode_kinds() -> Vec<EpisodeKind> {
    variants!(EpisodeKind:
        MemoryHigh => EpisodeKind::MemoryHigh,
        DiskBusy => EpisodeKind::DiskBusy { disk: string("0 C:") },
        CpuSaturated => EpisodeKind::CpuSaturated,
        DpcIsrHint => EpisodeKind::DpcIsrHint { core: 2 },
        DiskLatencyHint => EpisodeKind::DiskLatencyHint { disk: string("0 C:") },
    )
}

fn episode(full: bool) -> Episode {
    Episode {
        kind: EpisodeKind::MemoryHigh,
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_600_000,
        peak: 97.5,
        culprit: full.then(|| Culprit {
            pid: 4120,
            name: string("example.exe"),
            value: 1.5e9,
        }),
        truncated: full,
    }
}

fn summaries() -> Vec<OccurrenceSummary> {
    let mut all = Vec::new();
    for (index, &kind) in OccurrenceKind::ALL.iter().enumerate() {
        all.push(OccurrenceSummary {
            id: 1 + index as i64,
            kind,
            start_ms: 1_790_000_000_000,
            end_ms: 1_790_000_005_000,
            severity: Severity::ALL[index % Severity::ALL.len()],
            cause: Cause::Unclear,
        });
    }
    all.extend(causes().into_iter().map(|cause| OccurrenceSummary {
        id: 9,
        kind: OccurrenceKind::Freeze,
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_005_000,
        severity: Severity::Medium,
        cause,
    }));
    all
}

fn occurrence(full: bool) -> Occurrence {
    Occurrence {
        kind: OccurrenceKind::Freeze,
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_005_000,
        severity: Severity::Severe,
        triggers: first(triggers(), 2),
        window: window(full),
        suspects: small_suspects(),
        facts: first(fact_kinds(), 2)
            .into_iter()
            .map(|kind| Fact {
                at_ms: 1_790_000_001_000,
                kind,
            })
            .collect(),
        sensitive: full,
        truncated: full,
    }
}

fn columns() -> Columns {
    Columns {
        t: vec![1_790_000_000.0, 1_790_000_001.0],
        cpu: vec![Some(12.5), None],
        mem_used_pct: vec![Some(61.5), None],
        disk_active_max: vec![Some(7.5), None],
        disk_latency_max: vec![Some(1.5), None],
        gpu: vec![Some(30.0), None],
        probe_lag: vec![Some(0.5), None],
        state: vec![PcState::Calm, PcState::Suspended],
    }
}

fn test_span(kind: TestKind, outcome: Outcome) -> TestSpan {
    TestSpan {
        id: 5,
        kind,
        subject: Subject::Disk(string("0 C:")),
        started_ms: 1_790_000_000_000,
        ended_ms: 1_790_000_600_000,
        outcome,
    }
}

fn test_spans() -> Vec<TestSpan> {
    TestKind::ALL
        .iter()
        .zip(Outcome::ALL.iter().cycle())
        .map(|(&kind, &outcome)| test_span(kind, outcome))
        .collect()
}

fn capture_sources() -> Vec<CaptureSource> {
    variants!(CaptureSource:
        Ok => CaptureSource::Ok,
        Unavailable => CaptureSource::Unavailable(string("no access")),
        Partial => CaptureSource::Partial { covered_secs: 40, reason: string("stopped early") },
    )
}

fn routine_stats() -> RoutineStats {
    RoutineStats {
        count: 120,
        total_us: 54_000,
        max_us: 900,
        buckets: [1; BUCKETS],
    }
}

fn deep_capture() -> DeepCapture {
    DeepCapture {
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_060_000,
        kernel: CaptureSource::Ok,
        frames: CaptureSource::Partial {
            covered_secs: 30,
            reason: string("stopped early"),
        },
        lost_events: 3,
        drivers: vec![DriverLatency {
            module: string("example.sys"),
            dpc: routine_stats(),
            isr: routine_stats(),
        }],
        longest: routine_kinds()
            .into_iter()
            .map(|kind| LongRoutine {
                ts_ms: 1_790_000_000_000,
                core: 1,
                module: string("example.sys"),
                kind,
                us: 1500,
            })
            .collect(),
        apps: vec![AppFrames {
            name: string("example.exe"),
            pid: 4120,
            frames: 600,
            median_ms: 8.5,
            p99_ms: 30.5,
            worst_ms: 80.5,
            hitches: 12,
        }],
        hitches: vec![FrameHitch {
            ts_ms: 1_790_000_000_000,
            app: string("example.exe"),
            pid: 4120,
            frame_ms: 70.5,
            median_ms: 8.5,
        }],
        hiccups: vec![ProbeHiccup {
            ts_ms: 1_790_000_000_000,
            lag_ms: 250.5,
        }],
    }
}

fn capture_note() -> CaptureNote {
    CaptureNote {
        id: 4,
        start_ms: 1_790_000_000_000,
        end_ms: 1_790_000_060_000,
        suspects: small_suspects(),
    }
}

fn smart_attribute() -> SmartAttribute {
    SmartAttribute {
        id: 5,
        current: 100,
        worst: 99,
        raw: 12,
    }
}

fn nvme_health() -> NvmeHealth {
    NvmeHealth {
        critical_warning: 0,
        temperature_c: 41,
        available_spare_pct: 100,
        spare_threshold_pct: 10,
        percentage_used: 3,
        media_errors: 0,
    }
}

fn boot_time() -> BootTime {
    BootTime {
        ts_ms: 1_790_000_000_000,
        boot_ms: 45_000,
        main_path_ms: 30_000,
        post_boot_ms: 15_000,
    }
}

fn boot_delay(kind: DelayKind) -> BootDelay {
    BootDelay {
        ts_ms: 1_790_000_000_000,
        kind,
        name: string("example"),
        total_ms: 9000,
        degradation_ms: 4000,
        security: false,
    }
}

fn delay_kinds() -> Vec<DelayKind> {
    variants!(DelayKind:
        App => DelayKind::App,
        Driver => DelayKind::Driver,
        Service => DelayKind::Service,
        Phase => DelayKind::Phase,
        Device => DelayKind::Device,
    )
}

fn boot_report() -> BootReport {
    BootReport {
        boots: vec![boot_time()],
        delays: delay_kinds().into_iter().map(boot_delay).collect(),
        skipped: 2,
    }
}

fn measures() -> Vec<Measure> {
    let mut all = variants!(Measure:
        Count => Measure::Count(3),
        Percent => Measure::Percent(91.5),
        Days => Measure::Days(12),
        Seconds => Measure::Seconds(40.5),
        Bytes => Measure::Bytes(1_000_000),
        Media => Measure::Media(Media::Hdd),
    );
    all.extend(Media::ALL.iter().map(|&media| Measure::Media(media)));
    all
}

fn references() -> Vec<Reference> {
    variants!(Reference:
        AtMost => Reference::AtMost(Measure::Percent(80.0)),
        AtLeast => Reference::AtLeast(Measure::Bytes(5_000_000)),
        Is => Reference::Is(Measure::Media(Media::Ssd)),
    )
}

fn subjects() -> Vec<Subject> {
    variants!(Subject:
        Pc => Subject::Pc,
        Disk => Subject::Disk(string("0 C:")),
        Volume => Subject::Volume(string("C:\\")),
    )
}

fn health_facts() -> Vec<HealthFact> {
    let mut all = variants!(HealthFact:
        Attribute => HealthFact::Attribute(smart_attribute()),
        PredictFailure => HealthFact::PredictFailure,
        Nvme => HealthFact::Nvme(nvme_health()),
        Event => HealthFact::Event { ts_ms: 1_790_000_000_000, provider: string("example-provider"), id: 153, meaning: EventMeaning::DiskRetry },
        Space => HealthFact::Space { free_bytes: 5_000_000, total_bytes: 100_000_000 },
        PageFile => HealthFact::PageFile(PageFile { path: string("C:\\pagefile.sys"), size_bytes: 4_000_000, peak_bytes: 3_000_000 }),
        NoPageFile => HealthFact::NoPageFile,
        CommitPeak => HealthFact::CommitPeak { pct: 91.5 },
        MemoryHighDays => HealthFact::MemoryHighDays { days: 4 },
        Product => HealthFact::Product(SecurityProduct { name: string("Example Antivirus"), on: true }),
        Boot => HealthFact::Boot(boot_time()),
        Delay => HealthFact::Delay(boot_delay(DelayKind::App)),
        Failures => HealthFact::Failures { last_week: 5, weekly_before: 1.5 },
        SystemDisk => HealthFact::SystemDisk { disk: string("0 C:"), media: Media::Hdd },
        MemoryDiagnostic => HealthFact::MemoryDiagnostic { ts_ms: 1_790_000_000_000, bad_pages: Some(2) },
        MemoryQuickTest => HealthFact::MemoryQuickTest { ts_ms: 1_790_000_000_000, errors: 1, tested_bytes: 1_000_000 },
        SurfaceScan => HealthFact::SurfaceScan { ts_ms: 1_790_000_000_000, unreadable_blocks: 3, slow_blocks: 8, whole_disk: true },
        VolumeCheck => HealthFact::VolumeCheck { ts_ms: 1_790_000_000_000 },
    );
    all.push(HealthFact::MemoryDiagnostic {
        ts_ms: 1_790_000_000_000,
        bad_pages: None,
    });
    all.extend(
        delay_kinds()
            .into_iter()
            .map(|kind| HealthFact::Delay(boot_delay(kind))),
    );
    all.extend(Media::ALL.iter().map(|&media| HealthFact::SystemDisk {
        disk: string("0 C:"),
        media,
    }));
    all
}

fn concerns() -> Vec<Concern> {
    variants!(Concern: Attention => Concern::Attention, Critical => Concern::Critical)
}

fn findings() -> Vec<Finding> {
    let mut all = Vec::new();
    for (index, &kind) in FindingKind::ALL.iter().enumerate() {
        all.push(Finding {
            kind,
            subject: Subject::Pc,
            concern: Concern::Attention,
            value: Measure::Count(index as u32),
            reference: Reference::AtMost(Measure::Count(0)),
            evidence: Vec::new(),
            next_steps: Vec::new(),
        });
    }
    let kinds = FindingKind::ALL.iter().cycle();
    for (((subject, concern), (value, reference)), kind) in subjects()
        .into_iter()
        .zip(concerns().into_iter().cycle())
        .zip(measures().into_iter().zip(references().into_iter().cycle()))
        .zip(kinds)
    {
        all.push(Finding {
            kind: *kind,
            subject,
            concern,
            value,
            reference,
            evidence: first(health_facts(), 3),
            next_steps: first(next_steps(), 3),
        });
    }
    all
}

fn finding_dtos(full: bool) -> Vec<FindingDto> {
    findings()
        .into_iter()
        .enumerate()
        .map(|(index, finding)| FindingDto {
            id: 1 + index as i64,
            opened_ms: 1_790_000_000_000,
            updated_ms: 1_790_000_100_000,
            closed_ms: full.then_some(1_790_000_200_000),
            finding,
            first_value: full.then_some(Measure::Count(9)),
        })
        .collect()
}

fn health_dto(full: bool) -> HealthDto {
    HealthDto {
        open: first(finding_dtos(false), 2),
        closed: first(finding_dtos(true), 2),
        last_scan_ms: full.then_some(1_790_000_000_000),
        boot: full.then(boot_report),
    }
}

fn dump_culprits() -> Vec<DumpCulprit> {
    let mut all = variants!(DumpCulprit:
        Module => DumpCulprit::Module { name: string("example.sys"), confidence: Confidence::High, intermediary: false },
        Hardware => DumpCulprit::Hardware,
        App => DumpCulprit::App { name: string("example.exe") },
    );
    all.extend(
        Confidence::ALL
            .iter()
            .map(|&confidence| DumpCulprit::Module {
                name: string("example.sys"),
                confidence,
                intermediary: true,
            }),
    );
    all
}

fn faults() -> Vec<FaultDto> {
    variants!(FaultDto:
        BugCheck => FaultDto::BugCheck { code: 0x9f, params: [string("0x3"), string("0x4"), string("0x5"), string("0x6")] },
        Exception => FaultDto::Exception { code: 0xc000_0005, address: string("0x7ff612340000") },
    )
}

fn dump_dto(kind: DumpKind, full: bool) -> DumpDto {
    DumpDto {
        id: 7,
        path: string("C:\\Windows\\Minidump\\example.dmp"),
        kind,
        crash_ms: 1_790_000_000_000,
        file_ms: 1_790_000_001_000,
        size_bytes: 2_000_000,
        found_ms: 1_790_000_002_000,
        analyzed_ms: full.then_some(1_790_000_003_000),
        deleted_ms: full.then_some(1_790_000_004_000),
        fault: full.then(|| faults().remove(0)),
        fault_module: some(full, "example.sys"),
        stack_modules: vec![string("example.sys")],
        process: some(full, "example.exe"),
        module_count: full.then_some(120),
        culprit: full.then_some(DumpCulprit::Hardware),
    }
}

fn dumps() -> Vec<DumpDto> {
    let mut all: Vec<DumpDto> = DumpKind::ALL
        .iter()
        .map(|&kind| dump_dto(kind, true))
        .collect();
    all.push(dump_dto(DumpKind::KernelMini, false));
    for (fault, culprit) in faults().into_iter().zip(dump_culprits()) {
        let mut dump = dump_dto(DumpKind::KernelMini, true);
        dump.fault = Some(fault);
        dump.culprit = Some(culprit);
        all.push(dump);
    }
    all.extend(dump_culprits().into_iter().skip(2).map(|culprit| {
        let mut dump = dump_dto(DumpKind::App, true);
        dump.culprit = Some(culprit);
        dump
    }));
    all
}

fn dumps_dto(full: bool) -> DumpsDto {
    DumpsDto {
        dumps: first(dumps(), 2),
        space_bytes: 4_000_000,
        reports: vec![BugCheckDto {
            ts_ms: 1_790_000_000_000,
            code: 0x50,
            params: [string("0x1"), string("0x2"), string("0x3"), string("0x4")],
            dump_path: string(""),
        }],
        crash: full.then(|| CrashControlDto {
            dump_type: 7,
            dump_file: string("C:\\Windows\\MEMORY.DMP"),
            minidump_dir: string("C:\\Windows\\Minidump"),
            overwrite: true,
        }),
    }
}

fn setting_samples() -> Vec<Setting> {
    let mut all = variants!(Setting:
        Dword => Setting::Dword { hive: Hive::LocalMachine, key: string("SOFTWARE\\Example"), name: string("Enabled") },
        Key => Setting::Key { hive: Hive::CurrentUser, key: string("SOFTWARE\\Example") },
        Service => Setting::Service { name: string("Example") },
        PowerAc => Setting::PowerAc { subgroup: guid(1), setting: guid(2), scheme: Some(guid(3)) },
        ActiveScheme => Setting::ActiveScheme,
        Hibernation => Setting::Hibernation,
        StartupApproval => Setting::StartupApproval { hive: Hive::CurrentUser, list: StartupList::Run, name: string("Example") },
    );
    all.push(Setting::PowerAc {
        subgroup: guid(1),
        setting: guid(2),
        scheme: None,
    });
    all.extend(
        [
            StartupList::Run,
            StartupList::Run32,
            StartupList::StartupFolder,
        ]
        .into_iter()
        .map(|list| Setting::StartupApproval {
            hive: Hive::LocalMachine,
            list,
            name: string("Example"),
        }),
    );
    all
}

/// Both hives and every startup list, so the unit enums are covered by their own variants.
fn hives_and_lists() -> (Vec<Hive>, Vec<StartupList>) {
    (
        variants!(Hive: LocalMachine => Hive::LocalMachine, CurrentUser => Hive::CurrentUser),
        variants!(StartupList: Run => StartupList::Run, Run32 => StartupList::Run32, StartupFolder => StartupList::StartupFolder),
    )
}

fn values() -> Vec<Value> {
    variants!(Value:
        Absent => Value::Absent,
        Dword => Value::Dword(1),
        Present => Value::Present,
        Service => Value::Service { start: 2, delayed: true, running: false },
        Index => Value::Index(1),
        Scheme => Value::Scheme(guid(4)),
        On => Value::On(true),
        Bytes => Value::Bytes(vec![1, 2, 3]),
    )
}

fn setting_changes() -> Vec<SettingChange> {
    setting_samples()
        .into_iter()
        .zip(values().into_iter().cycle())
        .map(|(setting, from)| SettingChange {
            setting,
            from,
            to: Value::Absent,
        })
        .collect()
}

fn signals() -> Vec<Signal> {
    let mut all = variants!(Signal:
        Finding => Signal::Finding(FindingKind::SlowBoot),
        UnfinishedShutdowns => Signal::UnfinishedShutdowns { count: 3 },
        SleepFalls => Signal::SleepFalls { count: 2 },
        Stutters => Signal::Stutters { count: 6 },
        DpcIsrStalls => Signal::DpcIsrStalls { count: 4 },
        GpuDriverStalls => Signal::GpuDriverStalls { count: 1 },
        DiskBusy => Signal::DiskBusy { count: 5, culprit: Some(string("example.exe")) },
        CpuSaturated => Signal::CpuSaturated { count: 5, culprit: Some(string("example.exe")) },
        CaptureDriver => Signal::CaptureDriver { start_ms: 1_790_000_000_000, module: string("example.sys") },
        CaptureFrames => Signal::CaptureFrames { start_ms: 1_790_000_000_000, app: string("example.exe") },
        AppCrashes => Signal::AppCrashes { count: 3 },
        HiberFile => Signal::HiberFile { bytes: 8_000_000_000 },
        Monitors => Signal::Monitors { count: 2 },
        DumpsOff => Signal::DumpsOff,
        SlowStartup => Signal::SlowStartup { name: string("example.exe"), total_ms: 9000 },
    );
    all.extend([
        Signal::DiskBusy {
            count: 5,
            culprit: None,
        },
        Signal::CpuSaturated {
            count: 5,
            culprit: None,
        },
    ]);
    all.extend(FindingKind::ALL.iter().map(|&kind| Signal::Finding(kind)));
    all
}

fn statuses() -> Vec<Status> {
    variants!(Status:
        Ready => Status::Ready,
        AlreadySet => Status::AlreadySet,
        Unavailable => Status::Unavailable,
        Protected => Status::Protected,
    )
}

fn advice(full: bool) -> Vec<Advice> {
    TWEAKS
        .iter()
        .zip(statuses().into_iter().cycle())
        .zip(Applier::ALL.iter().cycle())
        .zip(Takes::ALL.iter().cycle())
        .map(|(((&tweak, status), &applier), &takes)| Advice {
            tweak,
            subject: some(full, "Example"),
            status,
            signals: if full {
                first(signals(), 3)
            } else {
                Vec::new()
            },
            changes: first(setting_changes(), 2),
            current: first(setting_samples(), 2)
                .into_iter()
                .zip(values())
                .map(|(setting, value)| SettingValue { setting, value })
                .collect(),
            applier,
            takes,
        })
        .collect()
}

fn restore_points() -> Vec<RestorePoint> {
    let mut all = variants!(RestorePoint:
        Created => RestorePoint::Created,
        Reused => RestorePoint::Reused { at_ms: Some(1_790_000_000_000) },
        Off => RestorePoint::Off,
        Failed => RestorePoint::Failed { error: 1460 },
        Unconfirmed => RestorePoint::Unconfirmed,
    );
    all.push(RestorePoint::Reused { at_ms: None });
    all
}

fn tweak_changes() -> Vec<TweakChange> {
    let mut all: Vec<TweakChange> = restore_points()
        .into_iter()
        .map(Some)
        .chain([None])
        .zip(TWEAKS.iter())
        .map(|(restore_point, &tweak)| TweakChange {
            tweak,
            subject: restore_point.as_ref().map(|_| string("Example")),
            undo_of: restore_point.as_ref().map(|_| 3),
            results: first(setting_changes(), 2)
                .into_iter()
                .enumerate()
                .map(|(index, change)| SettingResult {
                    change,
                    error: (index % 2 == 0).then_some(5),
                })
                .collect(),
            restore_point,
        })
        .collect();
    all.push(TweakChange {
        tweak: TWEAKS[0],
        subject: None,
        undo_of: None,
        results: Vec::new(),
        restore_point: None,
    });
    all
}

fn tweak_rows() -> Vec<TweakRowDto> {
    tweak_changes()
        .into_iter()
        .enumerate()
        .map(|(index, change)| TweakRowDto {
            id: 1 + index as i64,
            applied_ms: 1_790_000_000_000,
            undone_ms: (index % 2 == 0).then_some(1_790_000_100_000),
            applier: Applier::ALL[index % Applier::ALL.len()],
            change,
            helped: (index % 2 == 0).then_some(Helped {
                days: 7,
                before: 9,
                after: 2,
            }),
            outside: index % 2 == 1,
        })
        .collect()
}

fn run_as() -> Vec<RunAs> {
    let mut all = variants!(RunAs:
        System => RunAs::System,
        LocalService => RunAs::LocalService,
        NetworkService => RunAs::NetworkService,
        CurrentUser => RunAs::CurrentUser { elevated: true },
        OtherUser => RunAs::OtherUser,
        Unknown => RunAs::Unknown,
    );
    all.push(RunAs::CurrentUser { elevated: false });
    all
}

fn signatures() -> Vec<Signature> {
    variants!(Signature:
        Valid => Signature::Valid { signer: string("Example Corp") },
        Invalid => Signature::Invalid,
        Unsigned => Signature::Unsigned,
        Unknown => Signature::Unknown,
    )
}

fn process_facts() -> Vec<ProcessFactsDto> {
    let mut all = Vec::new();
    for (run_as, signature) in run_as().into_iter().zip(signatures().into_iter().cycle()) {
        all.push(ProcessFactsDto {
            started_ms: Some(1_790_000_000_000),
            threads: Some(12),
            processes: Some(2),
            path: Some(string("C:\\Example\\example.exe")),
            run_as,
            signature,
        });
    }
    all.push(ProcessFactsDto {
        started_ms: None,
        threads: None,
        processes: None,
        path: None,
        run_as: RunAs::Unknown,
        signature: Signature::Unknown,
    });
    all
}

fn regions() -> Vec<Region> {
    variants!(Region:
        Unread => Region::Unread,
        Read => Region::Read { worst_ms: 12.5 },
        Unreadable => Region::Unreadable,
    )
}

fn surface_scan(full: bool) -> SurfaceScan {
    SurfaceScan {
        disk: 0,
        media: Media::Hdd,
        disk_bytes: 1_000_000_000_000,
        sector_bytes: 512,
        from_bytes: 0,
        to_bytes: 1_000_000_000_000,
        bands: [4; BANDS],
        unreadable_blocks: 2,
        regions: regions(),
        worst: vec![SlowBlock {
            offset: 4096,
            ms: 480.5,
        }],
        unreadable: vec![8192],
        failure: some(full, "read failed"),
        outside_io: full,
    }
}

fn benchmark(full: bool) -> Benchmark {
    Benchmark {
        volume: string("C:\\"),
        bus: Bus::Nvme,
        media: Media::Ssd,
        file_bytes: 1_000_000_000,
        written_bytes: 2_000_000_000,
        results: BenchMode::ALL
            .iter()
            .map(|&mode| BenchResult {
                mode,
                mb_per_s: 1500.5,
                iops: 90_000.5,
                max_ms: 12.5,
            })
            .collect(),
        outside_io: full,
        failure: some(full, "no room"),
    }
}

fn volume_check(result: VolumeResult, full: bool) -> VolumeCheck {
    VolumeCheck {
        volume: string("C:\\"),
        exit_code: full.then_some(0),
        result,
        output: string("The file system is fine."),
        dirty_before: full.then_some(false),
        dirty_after: full.then_some(true),
    }
}

fn memory_quick(full: bool) -> MemoryQuick {
    MemoryQuick {
        total_bytes: 17_179_869_184,
        tested_bytes: 2_000_000_000,
        rounds: 2,
        passes: 3,
        error_count: 1,
        errors: PATTERNS
            .iter()
            .map(|&pattern| MemoryError {
                offset: 4096,
                pattern,
                expected: 0xff,
                actual: 0xfe,
            })
            .collect(),
        failure: some(full, "out of memory"),
    }
}

fn test_data() -> Vec<TestData> {
    let mut all = variants!(TestData:
        MemoryQuick => TestData::MemoryQuick(memory_quick(true)),
        DiskSurface => TestData::DiskSurface(surface_scan(true)),
        VolumeCheck => TestData::VolumeCheck(volume_check(VolumeResult::ProblemsFound, true)),
        DiskBenchmark => TestData::DiskBenchmark(benchmark(true)),
    );
    all.push(TestData::MemoryQuick(memory_quick(false)));
    all.push(TestData::DiskSurface(surface_scan(false)));
    all.push(TestData::DiskBenchmark(benchmark(false)));
    all.extend(VolumeResult::ALL.iter().flat_map(|&result| {
        [true, false].map(|full| TestData::VolumeCheck(volume_check(result, full)))
    }));
    all
}

fn test_runs() -> Vec<TestRun> {
    test_data()
        .into_iter()
        .zip(Outcome::ALL.iter().cycle())
        .zip(subjects().into_iter().cycle())
        .map(|((data, &outcome), subject)| TestRun {
            subject,
            started_ms: 1_790_000_000_000,
            ended_ms: 1_790_000_600_000,
            outcome,
            data,
        })
        .collect()
}

fn surface_progress() -> SurfaceProgress {
    SurfaceProgress {
        disk: 0,
        ts_ms: 1_790_000_000_000,
        at_bytes: 500_000_000_000,
        from_bytes: 0,
        disk_bytes: 1_000_000_000_000,
        bands: [4; BANDS],
        unreadable_blocks: 1,
    }
}

fn diagnostic_results() -> Vec<DiagnosticResult> {
    variants!(DiagnosticResult:
        Pass => DiagnosticResult::Pass,
        Cancelled => DiagnosticResult::Cancelled,
        Incomplete => DiagnosticResult::Incomplete,
        Errors => DiagnosticResult::Errors,
    )
}

fn diagnostics() -> Vec<MemoryDiagnostic> {
    diagnostic_results()
        .into_iter()
        .enumerate()
        .map(|(index, result)| MemoryDiagnostic {
            record_id: 800 + index as u64,
            ts_ms: 1_790_000_000_000,
            result,
            detail: (index % 2 == 0).then_some(DiagnosticDetail {
                memory_mb: 16_384,
                test_type: 10,
                duration_s: 900,
                test_count: 4,
                pages_tested: 4_000_000,
                pages_untested: 100,
                bad_pages: 2,
            }),
        })
        .collect()
}

fn running_test(full: bool) -> RunningTest {
    RunningTest {
        kind: TestKind::DiskSurface,
        subject: Subject::Disk(string("0 C:")),
        started_ms: 1_790_000_000_000,
        ends_ms: full.then_some(1_790_000_600_000),
        stop: Arc::new(AtomicBool::new(false)),
        stop_file: None,
    }
}

fn tests_dto(full: bool) -> TestsDto {
    TestsDto {
        memory: MemoryDto {
            total_bytes: 17_179_869_184,
            available_bytes: 8_589_934_592,
            quick_bytes: full.then_some(2_000_000_000),
            minutes: [2, 4, 8],
        },
        diagnostics: diagnostics(),
        disks: both(disk_dto),
        volumes: both(volume_dto),
        runs: first(test_runs(), 2)
            .into_iter()
            .enumerate()
            .map(|(index, run)| TestRunDto {
                id: 1 + index as i64,
                run,
            })
            .collect(),
        running: full.then(|| running_test(true)),
        scan: full.then(|| ScanUnderWay {
            stem: string("scan-0"),
            progress: surface_progress(),
        }),
        bench_file_bytes: 1_000_000_000,
        bench_needed_bytes: 2_000_000_000,
    }
}

fn disk_dto(full: bool) -> DiskDto {
    DiskDto {
        number: 0,
        name: string("0 C:"),
        media: Media::Ssd,
        bus: Bus::Nvme,
        bytes: full.then_some(1_000_000_000_000),
        volumes: vec![string("C:\\")],
        ranges: BenchMode::ALL
            .iter()
            .map(|&mode| BenchRange {
                mode,
                low: 500.5,
                high: 3500.5,
            })
            .collect(),
    }
}

fn volume_dto(full: bool) -> VolumeDto {
    VolumeDto {
        root: string("C:\\"),
        total_bytes: 1_000_000_000_000,
        free_bytes: 400_000_000_000,
        system: full,
        disk: full.then_some(0),
        file_system: some(full, "NTFS"),
    }
}

fn disk_health(full: bool) -> DiskHealth {
    DiskHealth {
        name: string("0 C:"),
        read_ms: 1_790_000_000_000,
        predict_failure: full,
        attributes: vec![smart_attribute()],
        nvme: full.then(nvme_health),
    }
}

fn nvme_detail() -> NvmeDetail {
    NvmeDetail {
        health: nvme_health(),
        data_read_bytes: 5_000_000_000,
        data_written_bytes: 4_000_000_000,
        power_cycles: 300,
        power_on_hours: 4000,
        unsafe_shutdowns: 7,
        error_log_entries: 0,
    }
}

fn now_frame(full: bool) -> NowFrame {
    NowFrame {
        sample: full.then(|| sample(true)),
        top: full.then(top),
        programs: full.then(top),
        hung: vec![HungWindow {
            pid: 4120,
            name: string("example.exe"),
        }],
        latest: full.then(|| summaries().remove(0)),
        pressure: episode_kinds(),
    }
}

fn live_point() -> LivePoint {
    LivePoint {
        reading: reading(true),
        state: PcState::Busy,
        lanes: Lanes {
            cpu: PcState::Calm,
            memory: PcState::Busy,
            disk: PcState::Pressure,
            gpu: PcState::Suspended,
        },
    }
}

fn agent_status(full: bool) -> AgentStatus {
    AgentStatus {
        paused: full,
        paused_until_ms: full.then_some(1_790_000_300_000),
        sensitive_until_ms: full.then_some(1_790_000_600_000),
        state: [TrayState::Normal, TrayState::Attention, TrayState::Problem][usize::from(full)],
        started_at_ms: 1_789_000_000_000,
        last_sample_ms: full.then_some(1_790_000_000_000),
        last_error: some(full, "pipe closed"),
        suggest_capture: full,
        cpu_pct: full.then_some(1.5),
    }
}

fn tray_states() -> Vec<TrayState> {
    variants!(TrayState:
        Normal => TrayState::Normal,
        Attention => TrayState::Attention,
        Problem => TrayState::Problem,
    )
}

fn ui_errors() -> Vec<UiError> {
    variants!(UiError:
        AgentStopped => UiError::AgentStopped,
        Mismatch => UiError::Mismatch,
        Refused => UiError::Refused(string("not allowed")),
        StartTimeout => UiError::StartTimeout,
        Store => UiError::Store(StoreError::NotFound),
        Ipc => UiError::Ipc(IpcError::AlreadyRunning),
        Failed => UiError::Failed(string("something failed")),
        Declined => UiError::Declined,
        ChangedSince => UiError::ChangedSince,
    )
}

fn settings() -> Vec<Settings> {
    let mut all = Vec::new();
    for (index, &language) in LanguagePref::ALL.iter().enumerate() {
        all.push(Settings {
            retention: Retention {
                samples_hours: 48,
                minutes_days: 30,
                records_days: 365,
            },
            language,
            theme: ThemePref::ALL[index % ThemePref::ALL.len()],
            color_theme: ColorTheme::ALL[index % ColorTheme::ALL.len()],
            detail_level: DetailLevel::ALL[index % DetailLevel::ALL.len()],
        });
    }
    let base = all[0];
    for (index, &theme) in ThemePref::ALL.iter().enumerate() {
        all.push(Settings {
            theme,
            color_theme: ColorTheme::ALL[ColorTheme::ALL.len() - 1 - index % ColorTheme::ALL.len()],
            ..base
        });
    }
    all.extend(ColorTheme::ALL.iter().map(|&color_theme| Settings {
        color_theme,
        ..base
    }));
    all.extend(DetailLevel::ALL.iter().map(|&detail_level| Settings {
        detail_level,
        ..base
    }));
    all
}

fn update_groups() -> Vec<Group> {
    vec![Group {
        kind: string("Added"),
        items: vec![string("One thing.")],
    }]
}

fn render() -> String {
    let mut wire = Wire::default();

    wire.add(TYPES, "PcState", PcState::ALL.to_vec());
    wire.add(
        TYPES,
        "HistoryState",
        variants!(HistoryState: Ready => HistoryState::Ready, Empty => HistoryState::Empty),
    );
    wire.add(
        TYPES,
        "Resolution",
        variants!(Resolution: Seconds => Resolution::Seconds, Minutes => Resolution::Minutes),
    );
    wire.add(TYPES, "Columns", vec![columns()]);
    wire.add(
        TYPES,
        "TimelineDto",
        history_states()
            .into_iter()
            .map(|(history, resolution)| TimelineDto {
                history,
                resolution,
                step_ms: 1000,
                columns: columns(),
            })
            .collect(),
    );
    wire.add(TYPES, "Reading", both(reading));
    wire.add(
        TYPES,
        "CoreLoad",
        vec![CoreLoad {
            cpu_pct: 10.0,
            dpc_pct: 0.5,
            isr_pct: 0.25,
        }],
    );
    wire.add(
        TYPES,
        "DiskLoad",
        [true, false]
            .into_iter()
            .map(|full| DiskLoad {
                name: string("0 C:"),
                active_pct: 7.5,
                latency_ms: full.then_some(1.5),
                queue_len: full.then_some(0.5),
                bytes_per_s: full.then_some(1024.0),
            })
            .collect(),
    );
    wire.add(TYPES, "Sample", both(sample));
    wire.add(TYPES, "ProcessSample", vec![process_sample()]);
    wire.add(TYPES, "ProcessTop", vec![top()]);
    wire.add(
        TYPES,
        "HungWindow",
        vec![HungWindow {
            pid: 4120,
            name: string("example.exe"),
        }],
    );
    wire.add(TYPES, "Severity", Severity::ALL.to_vec());
    wire.add(TYPES, "OccurrenceKind", OccurrenceKind::ALL.to_vec());
    wire.add(TYPES, "Confidence", Confidence::ALL.to_vec());
    wire.add(TYPES, "Channel", Channel::ALL.to_vec());
    wire.add(TYPES, "HangOutcome", HangOutcome::ALL.to_vec());
    wire.add(TYPES, "GapReason", GapReason::ALL.to_vec());
    wire.add(TYPES, "Media", Media::ALL.to_vec());
    wire.add(TYPES, "Metric", Metric::ALL.to_vec());
    wire.add(TYPES, "EventMeaning", EventMeaning::ALL.to_vec());
    wire.add(TYPES, "NextStep", next_steps());
    wire.add(TYPES, "Cause", causes());
    wire.add(TYPES, "CrashKind", crash_kinds());
    wire.add(
        TYPES,
        "WinEvent",
        Channel::ALL
            .iter()
            .flat_map(|&channel| {
                EventMeaning::ALL
                    .iter()
                    .map(move |&meaning| win_event(channel, meaning))
            })
            .collect(),
    );
    wire.add(TYPES, "Evidence", evidence());
    wire.add(TYPES, "Suspect", suspects());
    wire.add(TYPES, "Trigger", triggers());
    wire.add(
        TYPES,
        "Gap",
        GapReason::ALL.iter().map(|&r| gap(r)).collect(),
    );
    wire.add(
        TYPES,
        "HangSpan",
        HangOutcome::ALL.iter().map(|&o| hang_span(o)).collect(),
    );
    wire.add(
        TYPES,
        "DiskInfo",
        Media::ALL
            .iter()
            .map(|&media| DiskInfo {
                name: string("0 C:"),
                media,
            })
            .collect(),
    );
    wire.add(TYPES, "MachineInfo", vec![machine()]);
    wire.add(TYPES, "Window", both(window));
    wire.add(TYPES, "FactKind", fact_kinds());
    wire.add(
        TYPES,
        "Fact",
        fact_kinds()
            .into_iter()
            .map(|kind| Fact {
                at_ms: 1_790_000_001_000,
                kind,
            })
            .collect(),
    );
    wire.add(TYPES, "OccurrenceSummary", summaries());
    wire.add(TYPES, "Occurrence", both(occurrence));
    wire.add(
        TYPES,
        "OccurrenceDto",
        both(|full| OccurrenceDto {
            id: 11,
            occurrence: occurrence(full),
            chart: columns(),
            capture: full.then(capture_note),
            tests: if full { test_spans() } else { Vec::new() },
        }),
    );
    wire.add(TYPES, "EpisodeKind", episode_kinds());
    wire.add(
        TYPES,
        "Culprit",
        vec![Culprit {
            pid: 4120,
            name: string("example.exe"),
            value: 1.5e9,
        }],
    );
    wire.add(
        TYPES,
        "Episode",
        episode_kinds()
            .into_iter()
            .flat_map(|kind| {
                both(episode).into_iter().map(move |mut episode| {
                    episode.kind = kind.clone();
                    episode
                })
            })
            .collect(),
    );
    wire.add(
        TYPES,
        "EpisodeDetailDto",
        history_states()
            .into_iter()
            .map(|(history, resolution)| EpisodeDetailDto {
                history,
                resolution,
                step_ms: 1000,
                from_ms: 1_789_999_000_000,
                to_ms: 1_790_000_900_000,
                chart: columns(),
                programs: vec![process_sample()],
                occurrences: first(summaries(), 2),
            })
            .collect(),
    );
    wire.add(
        TYPES,
        "MarksDto",
        history_states()
            .into_iter()
            .map(|(history, _)| MarksDto {
                history,
                occurrences: first(summaries(), 2),
                episodes: both(episode),
                by_test: vec![true, false],
                gaps: GapReason::ALL.iter().map(|&r| gap(r)).collect(),
                tests: test_spans(),
            })
            .collect(),
    );
    wire.add(TYPES, "TrayState", tray_states());
    wire.add(TYPES, "AgentStatus", both(agent_status));
    wire.add(
        TYPES,
        "AgentLink",
        variants!(AgentLink:
            Running => AgentLink::Running(agent_status(true)),
            Stopped => AgentLink::Stopped,
            Mismatch => AgentLink::Mismatch,
        )
        .into_iter()
        .chain([AgentLink::Running(agent_status(false))])
        .collect(),
    );
    wire.add(
        TYPES,
        "Retention",
        vec![Retention {
            samples_hours: 48,
            minutes_days: 30,
            records_days: 365,
        }],
    );
    wire.add(TYPES, "LanguagePref", LanguagePref::ALL.to_vec());
    wire.add(TYPES, "ThemePref", ThemePref::ALL.to_vec());
    wire.add(TYPES, "ColorTheme", ColorTheme::ALL.to_vec());
    wire.add(TYPES, "DetailLevel", DetailLevel::ALL.to_vec());
    wire.add(TYPES, "Settings", settings());
    wire.add(
        TYPES,
        "SettingsDto",
        settings()
            .into_iter()
            .take(2)
            .zip([Some(true), None])
            .map(|(settings, autostart)| SettingsDto {
                settings,
                autostart,
                editable: autostart.is_some(),
            })
            .chain([SettingsDto {
                settings: settings().remove(0),
                autostart: Some(false),
                editable: false,
            }])
            .collect(),
    );
    wire.add(
        TYPES,
        "NowFrame",
        both(now_frame)
            .into_iter()
            .chain(std::iter::once(NowFrame::default()))
            .collect(),
    );
    wire.add(TYPES, "Lanes", vec![live_point().lanes]);
    wire.add(TYPES, "LivePoint", vec![live_point()]);
    wire.add(
        TYPES,
        "LiveMessage",
        variants!(LiveMessage:
            Recent => LiveMessage::Recent { points: vec![live_point()] },
            Now => LiveMessage::Now { point: Some(live_point()), frame: Box::new(now_frame(true)) },
            Stopped => LiveMessage::Stopped,
        )
        .into_iter()
        .chain([LiveMessage::Now {
            point: None,
            frame: Box::new(now_frame(false)),
        }])
        .collect(),
    );
    wire.add(TYPES, "UiError", ui_errors());
    wire.add(
        TYPES,
        "UiErrorKind",
        ui_errors()
            .iter()
            .map(|error| serde_json::to_value(error).unwrap()["kind"].clone())
            .collect(),
    );
    wire.add(TYPES, "RoutineKind", routine_kinds());
    wire.add(TYPES, "RoutineStats", vec![routine_stats()]);
    wire.add(
        TYPES,
        "DriverLatency",
        vec![DriverLatency {
            module: string("example.sys"),
            dpc: routine_stats(),
            isr: routine_stats(),
        }],
    );
    wire.add(
        TYPES,
        "LongRoutine",
        routine_kinds()
            .into_iter()
            .map(|kind| LongRoutine {
                ts_ms: 1_790_000_000_000,
                core: 1,
                module: string("example.sys"),
                kind,
                us: 1500,
            })
            .collect(),
    );
    wire.add(TYPES, "AppFrames", deep_capture().apps);
    wire.add(TYPES, "FrameHitch", deep_capture().hitches);
    wire.add(TYPES, "ProbeHiccup", deep_capture().hiccups);
    wire.add(TYPES, "CaptureSource", capture_sources());
    wire.add(
        TYPES,
        "CaptureSummary",
        vec![CaptureSummary {
            id: 4,
            start_ms: 1_790_000_000_000,
            end_ms: 1_790_000_060_000,
        }],
    );
    wire.add(
        TYPES,
        "CaptureDto",
        capture_sources()
            .into_iter()
            .map(|kernel| CaptureDto {
                id: 4,
                capture: DeepCapture {
                    kernel,
                    ..deep_capture()
                },
                findings: small_suspects(),
            })
            .collect(),
    );
    wire.add(TYPES, "CaptureNote", vec![capture_note()]);
    wire.add(
        TYPES,
        "CaptureRun",
        vec![CaptureRun {
            started_ms: 1_790_000_000_000,
            ends_ms: 1_790_000_060_000,
        }],
    );
    wire.add(
        TYPES,
        "CaptureFinished",
        [Some(0), None]
            .map(|exit_code| CaptureFinished {
                ids: vec![4, 5],
                exit_code,
            })
            .to_vec(),
    );
    wire.add(TYPES, "Concern", concerns());
    wire.add(TYPES, "FindingKind", FindingKind::ALL.to_vec());
    wire.add(TYPES, "Subject", subjects());
    wire.add(TYPES, "Measure", measures());
    wire.add(TYPES, "Reference", references());
    wire.add(TYPES, "SmartAttribute", vec![smart_attribute()]);
    wire.add(TYPES, "NvmeHealth", vec![nvme_health()]);
    wire.add(
        TYPES,
        "PageFile",
        vec![PageFile {
            path: string("C:\\pagefile.sys"),
            size_bytes: 4_000_000,
            peak_bytes: 3_000_000,
        }],
    );
    wire.add(
        TYPES,
        "SecurityProduct",
        [true, false]
            .map(|on| SecurityProduct {
                name: string("Example Antivirus"),
                on,
            })
            .to_vec(),
    );
    wire.add(TYPES, "DelayKind", delay_kinds());
    wire.add(TYPES, "BootTime", vec![boot_time()]);
    wire.add(
        TYPES,
        "BootDelay",
        delay_kinds().into_iter().map(boot_delay).collect(),
    );
    wire.add(TYPES, "BootReport", vec![boot_report()]);
    wire.add(TYPES, "HealthFact", health_facts());
    wire.add(TYPES, "FindingDto", {
        let mut all = finding_dtos(true);
        all.extend(finding_dtos(false));
        all
    });
    wire.add(TYPES, "HealthDto", both(health_dto));
    wire.add(
        TYPES,
        "ScanFinished",
        [Some(0), None]
            .map(|exit_code| ScanFinished {
                scans: vec![1, 2],
                exit_code,
            })
            .to_vec(),
    );
    wire.add(TYPES, "DumpKind", DumpKind::ALL.to_vec());
    wire.add(TYPES, "FaultDto", faults());
    wire.add(TYPES, "DumpCulprit", dump_culprits());
    wire.add(TYPES, "DumpDto", dumps());
    wire.add(
        TYPES,
        "BugCheckDto",
        vec![BugCheckDto {
            ts_ms: 1_790_000_000_000,
            code: 0x50,
            params: [string("0x1"), string("0x2"), string("0x3"), string("0x4")],
            dump_path: string("C:\\Windows\\MEMORY.DMP"),
        }],
    );
    wire.add(
        TYPES,
        "CrashControlDto",
        vec![CrashControlDto {
            dump_type: 7,
            dump_file: string("C:\\Windows\\MEMORY.DMP"),
            minidump_dir: string("C:\\Windows\\Minidump"),
            overwrite: false,
        }],
    );
    wire.add(TYPES, "DumpsDto", both(dumps_dto));
    wire.add(
        TYPES,
        "DumpScanFinished",
        [Some(0), None]
            .map(|exit_code| DumpScanFinished {
                dumps: vec![7],
                exit_code,
            })
            .to_vec(),
    );
    wire.add(
        TYPES,
        "DeleteFinished",
        [Some(0), None]
            .map(|exit_code| DeleteFinished {
                deleted: 2,
                exit_code,
            })
            .to_vec(),
    );
    wire.add(TYPES, "TweakId", TWEAKS.to_vec());
    wire.add(TYPES, "Hive", hives_and_lists().0);
    wire.add(TYPES, "StartupList", hives_and_lists().1);
    wire.add(TYPES, "TweakSetting", setting_samples());
    wire.add(TYPES, "TweakValue", values());
    wire.add(TYPES, "SettingChange", setting_changes());
    wire.add(TYPES, "TweakSignal", signals());
    wire.add(TYPES, "TweakStatus", statuses());
    wire.add(TYPES, "Applier", Applier::ALL.to_vec());
    wire.add(TYPES, "Takes", Takes::ALL.to_vec());
    wire.add(TYPES, "Advice", {
        let mut all = advice(true);
        all.extend(advice(false));
        all
    });
    wire.add(
        TYPES,
        "SettingValue",
        setting_samples()
            .into_iter()
            .zip(values().into_iter().cycle())
            .map(|(setting, value)| SettingValue { setting, value })
            .collect(),
    );
    wire.add(
        TYPES,
        "SettingResult",
        setting_changes()
            .into_iter()
            .flat_map(|change| {
                [Some(5), None].map(|error| SettingResult {
                    change: change.clone(),
                    error,
                })
            })
            .collect(),
    );
    wire.add(TYPES, "RestorePoint", restore_points());
    wire.add(TYPES, "TweakChange", tweak_changes());
    wire.add(
        TYPES,
        "Helped",
        vec![Helped {
            days: 7,
            before: 9,
            after: 2,
        }],
    );
    wire.add(TYPES, "TweakRow", tweak_rows());
    wire.add(
        TYPES,
        "TweaksDto",
        both(|full| TweaksDto {
            advice: first(advice(full), 2),
            history: first(tweak_rows(), 2),
            last_restore_point_ms: full.then_some(1_790_000_000_000),
        }),
    );
    wire.add(
        TYPES,
        "TweaksApplied",
        [Some(0), None]
            .map(|exit_code| TweaksApplied {
                ids: vec![3, 4],
                exit_code,
            })
            .to_vec(),
    );
    wire.add(TYPES, "NvmeDetail", vec![nvme_detail()]);
    wire.add(TYPES, "RunAs", run_as());
    wire.add(TYPES, "Signature", signatures());
    wire.add(TYPES, "ProcessFacts", process_facts());
    wire.add(
        TYPES,
        "ProgramMarks",
        vec![ProgramMarksDto {
            occurrences: first(summaries(), 2),
            episodes: both(episode),
        }],
    );
    wire.add(
        TYPES,
        "ProgramPoint",
        vec![ProgramPoint {
            ts_ms: 1_790_000_000_000,
            cpu_pct: 14.5,
            private_bytes: 123_456_789,
            io_bps: 4096.0,
        }],
    );
    wire.add(
        TYPES,
        "SmartPoint",
        vec![SmartPointDto {
            read_ms: 1_790_000_000_000,
            current: 100,
            worst: 99,
            raw: 12,
        }],
    );
    wire.add(TYPES, "DiskHealth", both(disk_health));
    wire.add(TYPES, "TestKind", TestKind::ALL.to_vec());
    wire.add(TYPES, "TestOutcome", Outcome::ALL.to_vec());
    wire.add(TYPES, "Pattern", PATTERNS.to_vec());
    wire.add(
        TYPES,
        "MemoryError",
        PATTERNS
            .iter()
            .map(|&pattern| MemoryError {
                offset: 4096,
                pattern,
                expected: 0xff,
                actual: 0xfe,
            })
            .collect(),
    );
    wire.add(TYPES, "MemoryQuick", both(memory_quick));
    wire.add(TYPES, "Region", regions());
    wire.add(
        TYPES,
        "SlowBlock",
        vec![SlowBlock {
            offset: 4096,
            ms: 480.5,
        }],
    );
    wire.add(TYPES, "SurfaceScan", both(surface_scan));
    wire.add(TYPES, "Bus", Bus::ALL.to_vec());
    wire.add(TYPES, "BenchMode", BenchMode::ALL.to_vec());
    wire.add(
        TYPES,
        "BenchResult",
        BenchMode::ALL
            .iter()
            .map(|&mode| BenchResult {
                mode,
                mb_per_s: 1500.5,
                iops: 90_000.5,
                max_ms: 12.5,
            })
            .collect(),
    );
    wire.add(TYPES, "Benchmark", both(benchmark));
    wire.add(TYPES, "VolumeResult", VolumeResult::ALL.to_vec());
    wire.add(
        TYPES,
        "VolumeCheck",
        VolumeResult::ALL
            .iter()
            .flat_map(|&result| both(move |full| volume_check(result, full)))
            .collect(),
    );
    wire.add(TYPES, "TestData", test_data());
    wire.add(TYPES, "TestRun", test_runs());
    wire.add(
        TYPES,
        "TestRunDto",
        test_runs()
            .into_iter()
            .enumerate()
            .map(|(index, run)| TestRunDto {
                id: 1 + index as i64,
                run,
            })
            .collect(),
    );
    wire.add(TYPES, "TestSpan", test_spans());
    wire.add(TYPES, "DiagnosticResult", diagnostic_results());
    wire.add(
        TYPES,
        "DiagnosticDetail",
        vec![DiagnosticDetail {
            memory_mb: 16_384,
            test_type: 10,
            duration_s: 900,
            test_count: 4,
            pages_tested: 4_000_000,
            pages_untested: 100,
            bad_pages: 2,
        }],
    );
    wire.add(TYPES, "MemoryDiagnostic", diagnostics());
    wire.add(
        TYPES,
        "MemoryDto",
        both(|full| MemoryDto {
            total_bytes: 17_179_869_184,
            available_bytes: 8_589_934_592,
            quick_bytes: full.then_some(2_000_000_000),
            minutes: [2, 4, 8],
        }),
    );
    wire.add(TYPES, "DiskDto", both(disk_dto));
    wire.add(TYPES, "VolumeDto", both(volume_dto));
    wire.add(TYPES, "SurfaceProgress", vec![surface_progress()]);
    wire.add(
        TYPES,
        "ScanUnderWay",
        vec![ScanUnderWay {
            stem: string("scan-0"),
            progress: surface_progress(),
        }],
    );
    wire.add(TYPES, "RunningTest", both(running_test));
    wire.add(TYPES, "TestsDto", both(tests_dto));
    wire.add(
        TYPES,
        "DiskDetailDto",
        vec![
            DiskDetailDto {
                health: Some(disk_health(true)),
                nvme: Some(nvme_detail()),
            },
            DiskDetailDto {
                health: None,
                nvme: None,
            },
        ],
    );
    wire.add(
        TYPES,
        "TestProgress",
        PATTERNS
            .iter()
            .map(|&pattern| TestProgress::Memory {
                pattern,
                rounds: 2,
                passes: 3,
                error_count: 1,
            })
            .chain(
                BenchMode::ALL
                    .iter()
                    .map(|&mode| TestProgress::Benchmark { mode }),
            )
            .chain([TestProgress::Surface(surface_progress())])
            .chain(variants!(TestProgress:
                Memory => TestProgress::Memory { pattern: PATTERNS[0], rounds: 1, passes: 1, error_count: 0 },
                Benchmark => TestProgress::Benchmark { mode: BenchMode::SeqRead },
                Surface => TestProgress::Surface(surface_progress()),
            ))
            .collect(),
    );
    wire.add(
        TYPES,
        "TestFinished",
        first(test_runs(), 3)
            .into_iter()
            .map(|run| TestFinished {
                id: Some(8),
                run: Some(run),
                exit_code: Some(0),
            })
            .chain([TestFinished {
                id: None,
                run: None,
                exit_code: None,
            }])
            .collect(),
    );

    wire.add(
        UPDATE,
        "UpdatePrefs",
        vec![
            UpdatePrefs {
                check: Some(true),
                last_check_ms: Some(1_790_000_000_000),
                seen_version: Some(string("0.1.0")),
            },
            UpdatePrefs::default(),
        ],
    );
    wire.add(UPDATE, "NotesGroup", update_groups());
    wire.add(
        UPDATE,
        "Release",
        [Some(string("2026-11-10")), None]
            .map(|date| Release {
                version: string("0.2.0"),
                date,
                groups: update_groups(),
            })
            .to_vec(),
    );
    wire.add(
        UPDATE,
        "CheckResult",
        variants!(CheckResult:
            UpToDate => CheckResult::UpToDate,
            Available => CheckResult::Available { version: string("0.2.0"), date_ms: Some(1_790_000_000_000), groups: update_groups() },
        )
        .into_iter()
        .chain([CheckResult::Available {
            version: string("0.2.0"),
            date_ms: None,
            groups: Vec::new(),
        }])
        .collect(),
    );
    wire.add(
        UPDATE,
        "Progress",
        variants!(Progress:
            Chunk => Progress::Chunk { done: 10, total: Some(100) },
            Installing => Progress::Installing,
            Failed => Progress::Failed { message: string("download failed") },
        )
        .into_iter()
        .chain([Progress::Chunk {
            done: 10,
            total: None,
        }])
        .collect(),
    );
    wire.add(
        ABOUT,
        "About",
        [Some(1_000_000), None]
            .map(|data_bytes| AboutDto {
                version: "0.1.0",
                data_bytes,
            })
            .to_vec(),
    );

    wire.text()
}

fn history_states() -> Vec<(HistoryState, Resolution)> {
    vec![
        (HistoryState::Ready, Resolution::Seconds),
        (HistoryState::Ready, Resolution::Minutes),
        (HistoryState::Empty, Resolution::Seconds),
    ]
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

/// The names `lib/types.ts` exports as types.
fn exported_types() -> BTreeSet<String> {
    let source = include_str!("../../src/lib/types.ts");
    source
        .lines()
        .filter_map(|line| {
            let rest = line
                .strip_prefix("export type ")
                .or_else(|| line.strip_prefix("export interface "))?;
            let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
            Some(rest[..end].to_owned())
        })
        .collect()
}

#[test]
fn the_fixture_file_matches_what_rust_sends() {
    let text = render();
    let path = fixture_path();
    if std::env::var_os("UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, &text).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    if committed != text {
        let line = committed
            .lines()
            .zip(text.lines())
            .position(|(old, new)| old != new)
            .unwrap_or_else(|| committed.lines().count().min(text.lines().count()));
        panic!(
            "the wire fixtures are stale (first difference at line {}): run \
             `UPDATE_FIXTURES=1 cargo test -p vestigium-ui wire` and commit {}",
            line + 1,
            FIXTURE
        );
    }
}

#[test]
fn every_type_the_window_declares_has_samples() {
    let text = render();
    let missing: Vec<String> = exported_types()
        .into_iter()
        .filter(|name| !SENT_ONLY.contains(&name.as_str()))
        .filter(|name| !text.contains(&format!("] satisfies {name}[];")))
        .collect();
    assert!(
        missing.is_empty(),
        "no sample for {missing:?}: add one to `render` in the wire test"
    );
}
