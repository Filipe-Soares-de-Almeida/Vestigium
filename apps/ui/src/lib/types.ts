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
export type OccurrenceKind = "Freeze" | "Stutter" | "AppHang" | "Reported";
export type Confidence = "Low" | "Medium" | "High";
export type Channel = "System" | "Application";
export type HangOutcome = "Recovered" | "Closed";
export type GapReason = "Paused" | "Suspended" | "Stopped";
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
  | "ServiceCrash";

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
  | "UpdateDriver";

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
  | { Frames: { app: string } };

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
  | { Reported: { event: WinEvent } };

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
  | { Gap: { reason: GapReason } };

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
  events: WinEvent[];
  gaps: Gap[];
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
