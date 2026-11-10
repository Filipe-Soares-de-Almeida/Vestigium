//! Domain types and rules for Vestigium.

mod blackbox;
mod capture;
mod dump;
mod episode;
mod event;
mod facts;
mod freeze;
mod gate;
mod hang;
pub mod health;
pub mod hwtest;
mod machine;
mod minute;
mod occurrence;
mod postmortem;
mod probe;
mod process;
mod reading;
mod ring;
mod rules;
mod sample;
mod settings;
mod shutdown;
mod stutter;
mod trigger;
mod tweak;
mod tweak_v1;
mod window;
mod xml;

pub use blackbox::{
    Blackbox, ECHOING_FREEZE_MS, GAP_MARGIN_MS, HANG_ECHO_MS, Notable, Output, RING_MS,
    RecentState, Timing,
};
pub use capture::{
    AppFrames, BUCKET_BOUNDS_US, BUCKETS, CAPTURE_MINUTES_MAX, CAPTURE_VERSION,
    CAPTURE_VERSION_MIN, CaptureSource, DeepCapture, DriverLatency, FrameAggregator, FrameHitch,
    HITCH_FACTOR, HITCH_MIN_MS, HITCHES_KEPT, LONGEST_KEPT, LongRoutine, ModuleMap,
    ON_DEMAND_SHARE, ProbeHiccup, RoutineAggregator, RoutineKind, RoutineStats, capture_findings,
    capture_suspects, is_compositor, is_intermediary, is_kernel_core, renders_steadily,
    steady_only,
};
pub use dump::{
    Analysis, BugCheckReport, DUMP_DELETION_VERSION, DUMP_FALL_REACH_MS, DUMP_SCAN_VERSION,
    DUMP_VERSION, DumpCulprit, DumpDeletion, DumpKind, DumpRecord, DumpScan, Fault,
    bug_check_event, bug_check_report, dump_culprit, dump_of_fall, fault_address, kernel_dump_kind,
    listing_changes,
};
pub use episode::{
    Culprit, Episode, EpisodeDetector, EpisodeKind, HINT_EVERY_MS, PressureSpan, QUIET_MS,
};
pub use event::{
    CURATED, Channel, CuratedEvent, EventMeaning, WinEvent, classify, query_xml, query_xml_for,
};
pub use facts::{Fact, FactKind, facts};
pub use freeze::{FREEZE_MS, freeze};
pub use gate::{GateState, RunGate};
pub use hang::{HUNG_AFTER_MS, HangTracker, HungWindow};
pub use machine::{DiskInfo, MachineInfo, Media};
pub use minute::{DiskMinute, MINUTE_MS, MinuteRollup, MinuteSample, minute_of, summarize_minute};
pub use occurrence::{Occurrence, OccurrenceKind, OccurrenceSummary, same_program};
pub use postmortem::{POSTMORTEM_BEFORE_MS, POSTMORTEM_REACH_MS, postmortem};
pub use probe::{DEFAULT_PROBE_PERIOD, ProbeTick, SENSITIVE_FOR, SENSITIVE_PROBE_PERIOD};
pub use process::{
    CpuWindow, PROGRAM_FRAMES_MAX, ProcessCounters, ProcessSample, ProcessSampleV1, ProcessTop,
    ProcessTopV1, ProgramPoint, ProgramRing, RunAs, Signature, heaviest, last_seen, process_rates,
    programs, usage,
};
pub use reading::{BUSY_ABOVE, PcState, Reading, pc_state};
pub use ring::Ring;
pub use rules::{
    Cause, Confidence, Evidence, Metric, NextStep, Suspect, crosses_down, suspects, threshold,
};
pub use sample::{CoreLoad, DiskLoad, Sample, SampleV1};
pub use settings::{
    ColorTheme, DetailLevel, LanguagePref, Retention, SettingError, Settings, ThemePref,
};
pub use shutdown::{
    CrashKind, PowerLossDetail, SHUTTING_DOWN, power_loss_detail, unexpected_shutdown_ms,
};
pub use stutter::{BURST_COUNT, BURST_WINDOW_MS, HICCUP_MS, StutterDetector};
pub use trigger::{HangOutcome, Severity, Trigger, severity};
pub use tweak::{
    Advice, Applier, BIG_HIBERFILE, CaptureCauses, ERROR_CHANGED_SINCE,
    ERROR_INTERRUPTED_UNCHANGED, ERROR_INTERRUPTED_UNKNOWN, ERROR_INTERRUPTED_WRITTEN,
    ERROR_MAY_HAVE_CHANGED, ERROR_UNREADABLE, Guid, Helped, Hive, IntentItem, IntentSetting,
    PCIE_ASPM, PlanItem, RestorePoint, SCHEME_BALANCED, SCHEME_HIGH_PERFORMANCE,
    SCHEME_POWER_SAVER, SUB_PCIEXPRESS, SUB_USB, Setting, SettingChange, SettingResult,
    SettingStore, Signal, StartupEntry, StartupList, Status, TWEAK_CHANGE_VERSION,
    TWEAK_INTENT_VERSION, TWEAK_PLAN_VERSION, TWEAKS, Takes, TweakChange, TweakId, TweakInputs,
    TweakIntent, TweakPlan, TweakResults, USB_SELECTIVE_SUSPEND, Value, advise, apply_value,
    carry_out, changed_outside, check_item, disabled_approval, helped, intent_of,
    settle_interrupted, undo,
};
pub use tweak_v1::{SettingChangeV1, SettingResultV1, SettingV1, TweakChangeV1};
pub use window::{Gap, GapReason, HangSpan, Window};
