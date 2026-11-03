// What the window's Rust side sends, in the shapes serde writes: unit variants as strings,
// variants with fields as `{ Variant: { ... } }`, a missing value as `null`.

export type PcState = "calm" | "busy" | "pressure" | "suspended";
export type HistoryState = "ready" | "empty";
export type Resolution = "seconds" | "minutes";

/** One entry per point; `null` where nothing was watched. Times are Unix epoch seconds. */
export interface Columns {
  t: number[];
  cpu: (number | null)[];
  mem_used_pct: (number | null)[];
  disk_active_max: (number | null)[];
  disk_latency_max: (number | null)[];
  gpu: (number | null)[];
  probe_lag: (number | null)[];
  state: PcState[];
}

export interface TimelineDto extends Columns {
  history: HistoryState;
  resolution: Resolution;
  step_ms: number;
}

export interface Reading {
  ts_ms: number;
  cpu_pct: number;
  mem_used_pct: number | null;
  disk_active_pct: number;
  disk_latency_ms: number;
  gpu_pct: number | null;
  probe_lag_ms: number;
}

export interface CoreLoad {
  cpu_pct: number;
  dpc_pct: number;
  isr_pct: number;
}

export interface DiskLoad {
  name: string;
  active_pct: number;
  latency_ms: number;
  queue_len: number;
  bytes_per_s: number;
}

export interface Sample {
  ts_ms: number;
  cpu_pct: number;
  dpc_pct: number;
  isr_pct: number;
  perf_limit_pct: number;
  proc_perf_pct: number;
  mem_total_bytes: number;
  mem_avail_bytes: number;
  commit_bytes: number;
  commit_limit_bytes: number;
  hard_faults_per_s: number;
  gpu_pct: number | null;
  probe_max_lag_ms: number;
  sampler_lag_ms: number;
  cores: CoreLoad[];
  disks: DiskLoad[];
}

export interface ProcessSample {
  pid: number;
  name: string;
  cpu_pct: number;
  private_bytes: number;
  read_bps: number;
  write_bps: number;
  gpu_pct: number;
}

export interface ProcessTop {
  ts_ms: number;
  entries: ProcessSample[];
}

export interface HungWindow {
  pid: number;
  name: string;
}

export type Severity = "Light" | "Medium" | "Severe";
export type OccurrenceKind = "Freeze" | "Stutter" | "AppHang" | "Reported" | "PostMortem";
export type Confidence = "Low" | "Medium" | "High";
export type Channel = "System" | "Application";
export type HangOutcome = "Recovered" | "Closed";
export type GapReason = "Paused" | "Suspended" | "Stopped" | "Crashed";
export type Media = "Ssd" | "Hdd" | "Unknown";

export type Metric =
  | "DiskLatencyMs"
  | "DiskQueue"
  | "DiskActivePct"
  | "MemAvailPct"
  | "HardFaultsPerS"
  | "CpuPct"
  | "CoreDpcIsrPct"
  | "GpuPct"
  | "PerfLimitPct"
  | "PrivateBytes"
  | "IoBytesPerS";

export type EventMeaning =
  | "StorageReset"
  | "DiskRetry"
  | "DiskBadBlock"
  | "PagingError"
  | "DisplayReset"
  | "HardwareError"
  | "PowerLoss"
  | "UnexpectedShutdown"
  | "BugCheck"
  | "CpuFirmwareLimit"
  | "LowVirtualMemory"
  | "AppHang"
  | "AppCrash"
  | "ServiceCrash"
  | "NoCrashDump"
  | "MemoryDiagnostic";

export type NextStep =
  | "UpdateStorageDriver"
  | "CheckDiskCable"
  | "RunDiskTest"
  | "FindDiskHeavyApp"
  | "CloseMemoryHeavyApp"
  | "CheckCooling"
  | "CheckPowerPlan"
  | "UpdateGpuDriver"
  | "EnableSensitiveMode"
  | "DeepCapture"
  | "ReportToAppVendor"
  | "UpdateDriver"
  | "BackUpNow"
  | "ReplaceDisk"
  | "FreeDiskSpace"
  | "LetWindowsManagePageFile"
  | "CheckHardwareStability"
  | "KeepOneAntivirus"
  | "ReviewStartupApps"
  | "WaitForShutdown"
  | "MoveWindowsToSsd"
  | "RunMemoryDiagnostic"
  | "RunMemTest86"
  | "RepairVolume";

export type Cause =
  | { Storage: { disk: string | null } }
  | { DiskIo: { disk: string; hdd: boolean; process: string | null } }
  | { Memory: { process: string | null } }
  | { Cpu: { processes: string[] } }
  | { Driver: { core: number } }
  | "GpuDriver"
  | "Throttling"
  | { App: { name: string } }
  | "Unclear"
  | { DriverModule: { module: string; intermediary: boolean } }
  | { Frames: { app: string } }
  | { BlueScreen: { code: number } }
  | "Unresponsive"
  | "SleepTransition"
  | { PowerCut: { no_dump: boolean } }
  | "HardwareFault";

/** How the PC went down, from what Windows logged on the next boot. */
export type CrashKind =
  | { BlueScreen: { code: number; params: number[] } }
  | "PowerButton"
  | "WhileShuttingDown"
  | "WhileSleeping"
  | "NoTrace";

export interface WinEvent {
  ts_ms: number;
  channel: Channel;
  provider: string;
  id: number;
  record_id: number;
  level: number;
  meaning: EventMeaning;
  data_xml: string;
}

export type Evidence =
  | { Event: { ts_ms: number; provider: string; id: number; meaning: EventMeaning } }
  | {
      Metric: {
        ts_ms: number;
        metric: Metric;
        value: number;
        threshold: number;
        subject: string | null;
      };
    }
  | { Process: { pid: number; name: string; metric: Metric; value: number } }
  | { Hang: { name: string; duration_ms: number } }
  | {
      Routine: {
        module: string;
        kind: RoutineKind;
        count: number;
        max_us: number;
        at_least_100us: number;
        at_least_1ms: number;
        core_pct: number;
      };
    }
  | { LongRoutine: { ts_ms: number; module: string; kind: RoutineKind; us: number } }
  | {
      Frames: { app: string; frames: number; hitches: number; worst_ms: number; median_ms: number };
    }
  | { Hitch: { ts_ms: number; app: string; frame_ms: number } }
  | { Hiccup: { ts_ms: number; lag_ms: number } }
  | { CaptureLoss: { lost_events: number } };

export interface Suspect {
  cause: Cause;
  confidence: Confidence;
  evidence: Evidence[];
  next_steps: NextStep[];
}

export type Trigger =
  | { Freeze: { start_ms: number; end_ms: number; lag_ms: number } }
  | { Stutter: { start_ms: number; end_ms: number; count: number; worst_ms: number } }
  | {
      AppHang: {
        pid: number;
        name: string;
        start_ms: number;
        end_ms: number;
        outcome: HangOutcome;
      };
    }
  | { Reported: { event: WinEvent } }
  | { Crash: { last_ms: number; down_ms: number; kind: CrashKind } };

export interface Gap {
  start_ms: number;
  end_ms: number;
  reason: GapReason;
}

export interface HangSpan {
  pid: number;
  name: string;
  start_ms: number;
  end_ms: number;
  outcome: HangOutcome;
}

export interface DiskInfo {
  name: string;
  media: Media;
}

export interface MachineInfo {
  logical_cpus: number;
  mem_total_bytes: number;
  disks: DiskInfo[];
}

export interface Window {
  from_ms: number;
  to_ms: number;
  samples: Sample[];
  tops: ProcessTop[];
  events: WinEvent[];
  hangs: HangSpan[];
  gaps: Gap[];
  machine: MachineInfo;
}

export type FactKind =
  | { FreezeStarted: { duration_ms: number } }
  | "FreezeEnded"
  | { Stutter: { count: number; worst_ms: number } }
  | { AppHung: { name: string } }
  | { AppRecovered: { name: string } }
  | { AppClosedWhileHung: { name: string } }
  | { Event: { meaning: EventMeaning; provider: string; id: number } }
  | { Crossed: { metric: Metric; value: number; threshold: number; subject: string | null } }
  | { Busiest: { metric: Metric; name: string; value: number } }
  | { Gap: { reason: GapReason } }
  | "LastWatched"
  | { WentDown: { kind: CrashKind } };

export interface Fact {
  at_ms: number;
  kind: FactKind;
}

export interface OccurrenceSummary {
  id: number;
  kind: OccurrenceKind;
  start_ms: number;
  end_ms: number;
  severity: Severity;
  cause: Cause;
}

export interface Occurrence {
  kind: OccurrenceKind;
  start_ms: number;
  end_ms: number;
  severity: Severity;
  triggers: Trigger[];
  window: Window;
  suspects: Suspect[];
  facts: Fact[];
  sensitive: boolean;
  truncated: boolean;
}

export interface OccurrenceDto extends Occurrence {
  id: number;
  /** What a deep capture that covered this window adds. */
  capture: CaptureNote | null;
  /** Vestigium tests that ran during the window. */
  tests: TestSpan[];
  /** The window second by second. */
  chart: Columns;
}

export type EpisodeKind =
  | "MemoryHigh"
  | { DiskBusy: { disk: string } }
  | "CpuSaturated"
  | { DpcIsrHint: { core: number } }
  | { DiskLatencyHint: { disk: string } };

export interface Culprit {
  pid: number;
  name: string;
  value: number;
}

export interface Episode {
  kind: EpisodeKind;
  start_ms: number;
  end_ms: number;
  peak: number;
  culprit: Culprit | null;
  truncated: boolean;
}

export interface MarksDto {
  history: HistoryState;
  occurrences: OccurrenceSummary[];
  episodes: Episode[];
  /** For each episode, in order: whether a Vestigium test most likely caused it. */
  by_test: boolean[];
  events: WinEvent[];
  gaps: Gap[];
  tests: TestSpan[];
}

export type TrayState = "normal" | "attention" | "problem";

export interface AgentStatus {
  paused: boolean;
  sensitive_until_ms: number | null;
  state: TrayState;
  started_at_ms: number;
  last_sample_ms: number | null;
  last_error: string | null;
  /** Sensitive mode saw stutters worth a deep capture. */
  suggest_capture: boolean;
}

export type AgentLink =
  | ({ link: "running" } & AgentStatus)
  | { link: "stopped" }
  | { link: "mismatch" };

export interface Retention {
  samples_hours: number;
  minutes_days: number;
  records_days: number;
}

export type LanguagePref = "System" | "En" | "PtBr";
export type ThemePref = "System" | "Light" | "Dark";
export type ColorTheme = "Red" | "Graphite" | "Slate" | "Indigo";
export type DetailLevel = "Simple" | "Detailed" | "Technical";

export interface Settings {
  retention: Retention;
  language: LanguagePref;
  theme: ThemePref;
  color_theme: ColorTheme;
  detail_level: DetailLevel;
}

export interface SettingsDto {
  settings: Settings;
  /** `null` while the watcher is stopped. */
  autostart: boolean | null;
  /** Only a running watcher writes settings. */
  editable: boolean;
}

export interface NowFrame {
  sample: Sample | null;
  /** The busiest programs, each summing its processes under the pid of the busiest one. */
  top: ProcessTop | null;
  hung: HungWindow[];
  latest: OccurrenceSummary | null;
  /** CPU, disk and memory pressure episodes under way; hints are left out. */
  pressure: EpisodeKind[];
}

/** The state each lane would give the PC on its own; a lane with no reading is suspended. */
export interface Lanes {
  cpu: PcState;
  memory: PcState;
  disk: PcState;
  gpu: PcState;
}

/** One second of the live chart. */
export interface LivePoint {
  reading: Reading;
  state: PcState;
  lanes: Lanes;
}

export type LiveMessage =
  | { kind: "recent"; points: LivePoint[] }
  | { kind: "now"; point: LivePoint | null; frame: NowFrame }
  | { kind: "stopped" };

export type UiErrorKind =
  | "agent_stopped"
  | "mismatch"
  | "refused"
  | "start_timeout"
  | "store"
  | "ipc"
  | "failed"
  | "declined";

/** What a failed command rejects with; the window words it from `kind`. */
export interface UiError {
  kind: UiErrorKind;
  message: string;
}

export type RoutineKind = "Dpc" | "Isr";

/** DPC or ISR calls of one driver; `buckets` follow the bounds 1, 2, 3, 4, 5, 10, 25, 50, 100, 250, 500, 1000 us. */
export interface RoutineStats {
  count: number;
  total_us: number;
  max_us: number;
  buckets: number[];
}

export interface DriverLatency {
  module: string;
  dpc: RoutineStats;
  isr: RoutineStats;
}

export interface LongRoutine {
  ts_ms: number;
  core: number;
  module: string;
  kind: RoutineKind;
  us: number;
}

export interface AppFrames {
  name: string;
  pid: number;
  frames: number;
  median_ms: number;
  p99_ms: number;
  worst_ms: number;
  hitches: number;
}

export interface FrameHitch {
  ts_ms: number;
  app: string;
  pid: number;
  frame_ms: number;
  median_ms: number;
}

export interface ProbeHiccup {
  ts_ms: number;
  lag_ms: number;
}

/** Whether a part of the capture was measured, or why not. */
export type CaptureSource = "Ok" | { Unavailable: string };

export interface CaptureSummary {
  id: number;
  start_ms: number;
  end_ms: number;
}

/** A stored deep capture with what it names. */
export interface CaptureDto {
  id: number;
  start_ms: number;
  end_ms: number;
  kernel: CaptureSource;
  frames: CaptureSource;
  lost_events: number;
  drivers: DriverLatency[];
  longest: LongRoutine[];
  apps: AppFrames[];
  hitches: FrameHitch[];
  hiccups: ProbeHiccup[];
  findings: Suspect[];
}

/** What a deep capture that covered an occurrence adds to it. */
export interface CaptureNote {
  id: number;
  start_ms: number;
  end_ms: number;
  suspects: Suspect[];
}

/** A deep capture under way. */
export interface CaptureRun {
  started_ms: number;
  ends_ms: number;
}

/** Sent as the `capture-finished` event when the helper exits. */
export interface CaptureFinished {
  ids: number[];
  exit_code: number | null;
}

/** How much a health finding asks for attention. */
export type Concern = "Attention" | "Critical";

export type FindingKind =
  | "DiskSmart"
  | "LowSpace"
  | "PageFile"
  | "MemoryOften"
  | "DisplayResets"
  | "HardwareErrors"
  | "StorageErrors"
  | "Throttling"
  | "WindowsOnHdd"
  | "TwoAntivirus"
  | "SlowBoot"
  | "DirtyRestarts"
  | "ShutdownUnfinished"
  | "FailuresRising"
  | "NoCrashDump"
  | "MemoryTestErrors"
  | "DiskUnreadable"
  | "VolumeErrors";

/** What a finding is about: a disk such as "0 C:", a volume such as "C:\", or the whole PC. */
export type Subject = "Pc" | { Disk: string } | { Volume: string };

export type Measure =
  | { Count: number }
  | { Percent: number }
  | { Days: number }
  | { Seconds: number }
  | { Bytes: number }
  | { Media: Media };

/** The range a healthy PC stays in. */
export type Reference = { AtMost: Measure } | { AtLeast: Measure } | { Is: Measure };

export interface SmartAttribute {
  id: number;
  current: number;
  worst: number;
  raw: number;
}

export interface NvmeHealth {
  critical_warning: number;
  temperature_c: number;
  available_spare_pct: number;
  spare_threshold_pct: number;
  percentage_used: number;
  media_errors: number;
}

export interface PageFile {
  path: string;
  size_bytes: number;
  peak_bytes: number;
}

export interface SecurityProduct {
  name: string;
  on: boolean;
}

export type DelayKind = "App" | "Driver" | "Service" | "Phase" | "Device";

/** One boot Windows measured. */
export interface BootTime {
  ts_ms: number;
  boot_ms: number;
  main_path_ms: number;
  post_boot_ms: number;
}

/** A step that slowed a boot; `security` steps are never offered for removal. */
export interface BootDelay {
  ts_ms: number;
  kind: DelayKind;
  name: string;
  total_ms: number;
  degradation_ms: number;
  security: boolean;
}

export interface BootReport {
  boots: BootTime[];
  delays: BootDelay[];
}

export type HealthFact =
  | { Attribute: SmartAttribute }
  | "PredictFailure"
  | { Nvme: NvmeHealth }
  | { Event: { ts_ms: number; provider: string; id: number; meaning: EventMeaning } }
  | { Space: { free_bytes: number; total_bytes: number } }
  | { PageFile: PageFile }
  | "NoPageFile"
  | { CommitPeak: { pct: number } }
  | { MemoryHighDays: { days: number } }
  | { Product: SecurityProduct }
  | { Boot: BootTime }
  | { Delay: BootDelay }
  | { Failures: { last_week: number; weekly_before: number } }
  | { SystemDisk: { disk: string; media: Media } }
  | { MemoryDiagnostic: { ts_ms: number; bad_pages: number | null } }
  | { MemoryQuickTest: { ts_ms: number; errors: number; tested_bytes: number } }
  | { SurfaceScan: { ts_ms: number; unreadable_blocks: number; slow_blocks: number; whole_disk: boolean } }
  | { VolumeCheck: { ts_ms: number } };

/** One health finding; `closed_ms` is null while it still holds. */
export interface FindingDto {
  id: number;
  opened_ms: number;
  updated_ms: number;
  closed_ms: number | null;
  kind: FindingKind;
  subject: Subject;
  concern: Concern;
  value: Measure;
  reference: Reference;
  evidence: HealthFact[];
  next_steps: NextStep[];
}

/** The health page: open findings (critical first), those closed in 30 days, the last scan. */
export interface HealthDto {
  open: FindingDto[];
  closed: FindingDto[];
  last_scan_ms: number | null;
  boot: BootReport | null;
}

/** What a full scan returned once its helper exited. */
export interface ScanFinished {
  scans: number[];
  exit_code: number | null;
}

/** Where a dump came from, which says how much it can tell. */
export type DumpKind = "KernelMini" | "KernelMemory" | "LiveKernel" | "App";

/** What went wrong; 64-bit values come as hexadecimal text. */
export type FaultDto =
  | { BugCheck: { code: number; params: string[] } }
  | { Exception: { code: number; address: string } };

export type DumpCulprit =
  | { Module: { name: string; confidence: Confidence; intermediary: boolean } }
  | "Hardware"
  | { App: { name: string } };

/** One dump Windows kept; `analyzed_ms` is null until the helper read it. */
export interface DumpDto {
  id: number;
  path: string;
  kind: DumpKind;
  crash_ms: number;
  file_ms: number;
  size_bytes: number;
  found_ms: number;
  analyzed_ms: number | null;
  /** Null while the file is still there. */
  deleted_ms: number | null;
  fault: FaultDto | null;
  fault_module: string | null;
  stack_modules: string[];
  process: string | null;
  module_count: number | null;
  culprit: DumpCulprit | null;
}

/** A blue screen Windows reported on the next boot, read from its event. */
export interface BugCheckDto {
  ts_ms: number;
  code: number;
  params: string[];
  /** Empty when Windows saved no dump. */
  dump_path: string;
}

/** What Windows is set to write on a blue screen: 0 none, 1 complete, 2 kernel, 3 small, 7 automatic. */
export interface CrashControlDto {
  dump_type: number;
  dump_file: string;
  minidump_dir: string;
  overwrite: boolean;
}

/** The dumps page: dumps newest first, the space those on disk take, Windows' reports, the setting. */
export interface DumpsDto {
  dumps: DumpDto[];
  space_bytes: number;
  reports: BugCheckDto[];
  crash: CrashControlDto | null;
}

/** What reading the dumps returned once the helper exited. */
export interface DumpScanFinished {
  dumps: number[];
  exit_code: number | null;
}

/** What deleting dumps returned once the helper exited. */
export interface DeleteFinished {
  deleted: number;
  exit_code: number | null;
}

/** One Windows setting Vestigium may offer to change. */
export type TweakId =
  | "FastStartup"
  | "Hibernation"
  | "PowerPlan"
  | "PcieAspm"
  | "UsbSuspend"
  | "SysMain"
  | "WindowsSearch"
  | "GameDvr"
  | "GpuScheduling"
  | "Mpo"
  | "StartupApp"
  | "AppDumps"
  | "CrashDump";

export type Hive = "LocalMachine" | "CurrentUser";
export type StartupList = "Run" | "Run32" | "StartupFolder";

/** Something Windows keeps that a tweak reads and writes; GUIDs come as text. */
export type TweakSetting =
  | { Dword: { hive: Hive; key: string; name: string } }
  | { Key: { hive: Hive; key: string } }
  | { Service: { name: string } }
  | { PowerAc: { subgroup: string; setting: string } }
  | "ActiveScheme"
  | "Hibernation"
  | { StartupApproval: { hive: Hive; list: StartupList; name: string } };

/** What a setting holds; "Absent" is a value, key or approval that does not exist. */
export type TweakValue =
  | "Absent"
  | { Dword: number }
  | "Present"
  | { Service: { start: number; delayed: boolean; running: boolean } }
  | { Index: number }
  | { Scheme: string }
  | { On: boolean }
  | { Bytes: number[] };

export interface SettingChange {
  setting: TweakSetting;
  from: TweakValue;
  to: TweakValue;
}

/** One thing Vestigium observed that backs a tweak. */
export type TweakSignal =
  | { Finding: FindingKind }
  | { UnfinishedShutdowns: { count: number } }
  | { SleepFalls: { count: number } }
  | { Stutters: { count: number } }
  | { DpcIsrStalls: { count: number } }
  | { GpuDriverStalls: { count: number } }
  | { DiskBusy: { count: number; culprit: string | null } }
  | { CpuSaturated: { count: number; culprit: string | null } }
  | { CaptureDriver: { start_ms: number; module: string } }
  | { CaptureFrames: { start_ms: number; app: string } }
  | { AppCrashes: { count: number } }
  | { HiberFile: { bytes: number } }
  | { Monitors: { count: number } }
  | "DumpsOff"
  | { SlowStartup: { name: string; total_ms: number } };

export type TweakStatus = "Ready" | "AlreadySet" | "Unavailable" | "Protected";
export type Applier = "User" | "Elevated";
export type Takes = "Now" | "NextShutdown" | "NextSignIn" | "Restart";

/** One tweak as the page shows it: the change it would make and the signals behind it. */
export interface Advice {
  tweak: TweakId;
  /** The startup entry's name, for StartupApp. */
  subject: string | null;
  status: TweakStatus;
  /** Empty when nothing here says the tweak would help. */
  signals: TweakSignal[];
  changes: SettingChange[];
  /** What each setting of the tweak holds now; empty when none could be read. */
  current: SettingValue[];
  applier: Applier;
  takes: Takes;
}

export interface SettingValue {
  setting: TweakSetting;
  value: TweakValue;
}

export interface SettingResult {
  change: SettingChange;
  /** The Win32 error when the setting was not written. */
  error: number | null;
}

export type RestorePoint =
  | "Created"
  | { Reused: { at_ms: number | null } }
  | "Off"
  | { Failed: { error: number } }
  | "Unconfirmed";

export interface TweakChange {
  tweak: TweakId;
  subject: string | null;
  undo_of: number | null;
  results: SettingResult[];
  restore_point: RestorePoint | null;
}

/** The same signal counted in equal spans before and after a change. */
export interface Helped {
  days: number;
  before: number;
  after: number;
}

/** One change in the history; `outside` means Windows now holds something else. */
export interface TweakRow {
  id: number;
  applied_ms: number;
  undone_ms: number | null;
  applier: Applier;
  change: TweakChange;
  helped: Helped | null;
  outside: boolean;
}

/** The tweaks page: every tweak with its signals, the history and Windows' last restore point. */
export interface TweaksDto {
  advice: Advice[];
  history: TweakRow[];
  last_restore_point_ms: number | null;
}

export interface TweakPick {
  tweak: TweakId;
  subject: string | null;
}

/** What applying or undoing returned: the history rows written and the helper's exit code. */
export interface TweaksApplied {
  ids: number[];
  exit_code: number | null;
}

/** Everything of an NVMe disk's health log the Tests page shows. */
export interface NvmeDetail {
  health: NvmeHealth;
  data_read_bytes: number;
  data_written_bytes: number;
  power_cycles: number;
  power_on_hours: number;
  unsafe_shutdowns: number;
  error_log_entries: number;
}

/** One disk's SMART block and verdict, as the watcher reads it. */
export interface DiskHealth {
  name: string;
  read_ms: number;
  predict_failure: boolean;
  attributes: SmartAttribute[];
  nvme: NvmeHealth | null;
}

export type TestKind = "MemoryQuick" | "DiskSurface" | "VolumeCheck" | "DiskBenchmark";

/** How a test ended; an interrupted test still reports what it covered. */
export type TestOutcome = "Clean" | "Problems" | "Interrupted" | "Failed";

export type Pattern =
  | "OwnAddress"
  | "Random"
  | "Compare"
  | "SolidBits"
  | "Checkerboard"
  | "WalkingOnes"
  | "WalkingZeros";

/** A word that read back different; `offset` is inside the tested memory, not a physical address. */
export interface MemoryError {
  offset: number;
  pattern: Pattern;
  expected: number;
  actual: number;
}

export interface MemoryQuick {
  total_bytes: number;
  tested_bytes: number;
  rounds: number;
  passes: number;
  error_count: number;
  errors: MemoryError[];
  failure: string | null;
}

export type Region = "Unread" | { Read: { worst_ms: number } } | "Unreadable";

export interface SlowBlock {
  offset: number;
  ms: number;
}

/** A read-only surface scan; `bands` count blocks under 5, 20, 50, 150, 500 ms and above. */
export interface SurfaceScan {
  disk: number;
  media: Media;
  disk_bytes: number;
  sector_bytes: number;
  from_bytes: number;
  to_bytes: number;
  bands: number[];
  unreadable_blocks: number;
  regions: Region[];
  worst: SlowBlock[];
  unreadable: number[];
  failure: string | null;
}

export type Bus = "Sata" | "Nvme" | "Usb" | "Other";

export type BenchMode = "SeqRead" | "SeqWrite" | "RandRead32" | "RandWrite32" | "RandRead1" | "RandWrite1";

export interface BenchResult {
  mode: BenchMode;
  mb_per_s: number;
  iops: number;
  max_ms: number;
}

export interface Benchmark {
  volume: string;
  bus: Bus;
  media: Media;
  file_bytes: number;
  written_bytes: number;
  results: BenchResult[];
  /** Other programs used the disk meanwhile, so the numbers may be low. */
  outside_io: boolean;
  failure: string | null;
}

export type VolumeResult = "NoProblems" | "ProblemsFound" | "CouldNotCheck";

export interface VolumeCheck {
  volume: string;
  exit_code: number | null;
  result: VolumeResult;
  /** chkdsk's own text, for the technical level. */
  output: string;
}

export type TestData =
  | { MemoryQuick: MemoryQuick }
  | { DiskSurface: SurfaceScan }
  | { VolumeCheck: VolumeCheck }
  | { DiskBenchmark: Benchmark };

export interface TestRun {
  subject: Subject;
  started_ms: number;
  ended_ms: number;
  outcome: TestOutcome;
  data: TestData;
}

export interface TestRunDto extends TestRun {
  id: number;
}

/** A test that ran inside a span, as the timeline and the report mark it. */
export interface TestSpan {
  id: number;
  kind: TestKind;
  subject: Subject;
  started_ms: number;
  ended_ms: number;
  outcome: TestOutcome;
}

export type DiagnosticResult = "Pass" | "Cancelled" | "Incomplete" | "Errors";

export interface DiagnosticDetail {
  memory_mb: number;
  test_type: number;
  duration_s: number;
  test_count: number;
  pages_tested: number;
  pages_untested: number;
  bad_pages: number;
}

/** One run of the Windows Memory Diagnostic, from the System log. */
export interface MemoryDiagnostic {
  ts_ms: number;
  result: DiagnosticResult;
  detail: DiagnosticDetail | null;
}

export interface MemoryDto {
  total_bytes: number;
  available_bytes: number;
  /** `null` when too little is free for a quick test. */
  quick_bytes: number | null;
  minutes: number[];
}

export interface DiskDto {
  number: number;
  /** As the disk counters name it, such as "0 C:". */
  name: string;
  media: Media;
  bus: Bus;
  bytes: number | null;
  volumes: string[];
}

export interface VolumeDto {
  root: string;
  total_bytes: number;
  free_bytes: number;
  system: boolean;
  disk: number | null;
  file_system: string | null;
}

export interface SurfaceProgress {
  ts_ms: number;
  at_bytes: number;
  from_bytes: number;
  disk_bytes: number;
  bands: number[];
  unreadable_blocks: number;
}

/** A disk scan the helper still runs, from the progress file it rewrites. */
export interface ScanUnderWay {
  stem: string;
  progress: SurfaceProgress;
}

/** The test this window started, while it runs. */
export interface RunningTest {
  kind: TestKind;
  subject: Subject;
  started_ms: number;
  ends_ms: number | null;
}

/** The Tests page. */
export interface TestsDto {
  memory: MemoryDto;
  /** Newest first. */
  diagnostics: MemoryDiagnostic[];
  disks: DiskDto[];
  volumes: VolumeDto[];
  /** Oldest first. */
  runs: TestRunDto[];
  running: RunningTest | null;
  scan: ScanUnderWay | null;
  bench_file_bytes: number;
}

/** SMART or NVMe detail read on request: reading an idle HDD spins it up. */
export interface DiskDetailDto {
  health: DiskHealth | null;
  nvme: NvmeDetail | null;
}

/** What a running test reports, after each pattern, mode or few seconds. */
export type TestProgress =
  | { kind: "memory"; pattern: Pattern; rounds: number; passes: number; error_count: number }
  | { kind: "benchmark"; mode: BenchMode }
  | ({ kind: "surface" } & SurfaceProgress);

/** What the window hears when a test ends; `id` is null while the watcher is stopped. */
export interface TestFinished {
  id: number | null;
  run: TestRun | null;
  exit_code: number | null;
}
