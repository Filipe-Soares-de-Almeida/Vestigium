//! Each list of variants below must name every variant of its enum: the `match` stops compiling
//! when a variant is missing, and the assertion stops the test when `ALL` is not the same list.

use vestigium_core::health::FindingKind;
use vestigium_core::hwtest::{BenchMode, Bus, Outcome, PATTERNS, Pattern, TestKind, VolumeResult};
use vestigium_core::{
    Applier, Channel, ColorTheme, Confidence, DetailLevel, DumpKind, EventMeaning, GapReason,
    HangOutcome, LanguagePref, Media, Metric, OccurrenceKind, PcState, Severity, TWEAKS, Takes,
    ThemePref, TweakId,
};

macro_rules! lists_every_variant {
    ($test:ident, $all:expr, $ty:ident: $($variant:ident),+ $(,)?) => {
        #[test]
        fn $test() {
            let listed = [$($ty::$variant),+];
            let _exhaustive = |value: $ty| match value {
                $($ty::$variant => ()),+
            };
            assert_eq!($all.as_slice(), listed.as_slice());
        }
    };
}

lists_every_variant!(
    occurrence_kind_lists_every_variant,
    OccurrenceKind::ALL,
    OccurrenceKind: Freeze, Stutter, AppHang, Reported, PostMortem
);
lists_every_variant!(
    severity_lists_every_variant,
    Severity::ALL,
    Severity: Light, Medium, Severe
);
lists_every_variant!(
    channel_lists_every_variant,
    Channel::ALL,
    Channel: System, Application
);
lists_every_variant!(
    gap_reason_lists_every_variant,
    GapReason::ALL,
    GapReason: Paused, Suspended, Stopped, Crashed, ClockChanged
);
lists_every_variant!(
    event_meaning_lists_every_variant,
    EventMeaning::ALL,
    EventMeaning: StorageReset, DiskRetry, DiskBadBlock, PagingError, DisplayReset, HardwareError,
        PowerLoss, UnexpectedShutdown, BugCheck, CpuFirmwareLimit, LowVirtualMemory, AppHang,
        AppCrash, ServiceCrash, NoCrashDump, MemoryDiagnostic
);
lists_every_variant!(
    hang_outcome_lists_every_variant,
    HangOutcome::ALL,
    HangOutcome: Recovered, Closed, StillHung
);
lists_every_variant!(
    test_kind_lists_every_variant,
    TestKind::ALL,
    TestKind: MemoryQuick, DiskSurface, VolumeCheck, DiskBenchmark
);
lists_every_variant!(
    outcome_lists_every_variant,
    Outcome::ALL,
    Outcome: Clean, Problems, Interrupted, Failed
);
lists_every_variant!(
    confidence_lists_every_variant,
    Confidence::ALL,
    Confidence: Low, Medium, High
);
lists_every_variant!(
    metric_lists_every_variant,
    Metric::ALL,
    Metric: DiskLatencyMs, DiskQueue, DiskActivePct, MemAvailPct, HardFaultsPerS, CpuPct,
        CoreDpcIsrPct, GpuPct, PerfLimitPct, PrivateBytes, IoBytesPerS
);
lists_every_variant!(
    pc_state_lists_every_variant,
    PcState::ALL,
    PcState: Calm, Busy, Pressure, Suspended
);
lists_every_variant!(
    language_pref_lists_every_variant,
    LanguagePref::ALL,
    LanguagePref: System, En, PtBr
);
lists_every_variant!(
    theme_pref_lists_every_variant,
    ThemePref::ALL,
    ThemePref: System, Light, Dark
);
lists_every_variant!(
    color_theme_lists_every_variant,
    ColorTheme::ALL,
    ColorTheme: Red, Graphite, Slate, Indigo
);
lists_every_variant!(
    detail_level_lists_every_variant,
    DetailLevel::ALL,
    DetailLevel: Simple, Detailed, Technical
);
lists_every_variant!(
    media_lists_every_variant,
    Media::ALL,
    Media: Ssd, Hdd, Unknown
);
lists_every_variant!(
    bus_lists_every_variant,
    Bus::ALL,
    Bus: Sata, Nvme, Usb, Other
);
lists_every_variant!(
    volume_result_lists_every_variant,
    VolumeResult::ALL,
    VolumeResult: NoProblems, ProblemsFound, CouldNotCheck
);
lists_every_variant!(
    dump_kind_lists_every_variant,
    DumpKind::ALL,
    DumpKind: KernelMini, KernelMemory, LiveKernel, App
);
lists_every_variant!(
    applier_lists_every_variant,
    Applier::ALL,
    Applier: User, Elevated
);
lists_every_variant!(
    takes_lists_every_variant,
    Takes::ALL,
    Takes: Now, NextShutdown, NextSignIn, Restart
);

lists_every_variant!(
    tweak_catalog_lists_every_tweak,
    TWEAKS,
    TweakId: FastStartup, Hibernation, PowerPlan, PcieAspm, UsbSuspend, SysMain, WindowsSearch,
        GameDvr, GpuScheduling, Mpo, StartupApp, AppDumps, CrashDump
);
lists_every_variant!(
    bench_mode_lists_every_variant,
    BenchMode::ALL,
    BenchMode: SeqRead, SeqWrite, RandRead32, RandWrite32, RandRead1, RandWrite1
);
lists_every_variant!(
    memory_test_patterns_list_every_variant,
    PATTERNS,
    Pattern: OwnAddress, Random, Compare, SolidBits, Checkerboard, WalkingOnes, WalkingZeros
);
lists_every_variant!(
    finding_kind_lists_every_variant,
    FindingKind::ALL,
    FindingKind: DiskSmart, LowSpace, PageFile, MemoryOften, DisplayResets, HardwareErrors,
        StorageErrors, Throttling, WindowsOnHdd, TwoAntivirus, SlowBoot, DirtyRestarts,
        ShutdownUnfinished, FailuresRising, NoCrashDump, MemoryTestErrors, DiskUnreadable,
        VolumeErrors
);
