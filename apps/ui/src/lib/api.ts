import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  AgentLink,
  CaptureDto,
  CaptureRun,
  CaptureSummary,
  DeleteFinished,
  DiskDetailDto,
  DumpScanFinished,
  DumpsDto,
  Episode,
  EpisodeDetailDto,
  HealthDto,
  LiveMessage,
  MarksDto,
  OccurrenceDto,
  RunningTest,
  ScanFinished,
  Settings,
  ProcessFacts,
  ProgramMarks,
  ProgramPoint,
  SettingsDto,
  SmartPoint,
  TestsDto,
  TimelineDto,
  TweakPick,
  TweaksApplied,
  TweaksDto,
} from "./types";

// Every call rejects with a `UiError` from ./types.

/** History of `[fromMs, toMs)` in at most `maxPoints` points, peaks kept. */
export function timeline(fromMs: number, toMs: number, maxPoints: number): Promise<TimelineDto> {
  return invoke("timeline", { fromMs, toMs, maxPoints });
}

export function marks(fromMs: number, toMs: number): Promise<MarksDto> {
  return invoke("marks", { fromMs, toMs });
}

export function occurrence(id: number): Promise<OccurrenceDto | null> {
  return invoke("occurrence", { id });
}

export function episodeDetail(episode: Episode): Promise<EpisodeDetailDto> {
  return invoke("episode_detail", { episode });
}

/** Streams the last ten minutes, then one second a second; resolves to the function that stops this stream and no later one. */
export async function liveStart(onMessage: (message: LiveMessage) => void): Promise<() => Promise<void>> {
  const channel = new Channel<LiveMessage>();
  channel.onmessage = onMessage;
  const id = await invoke<number>("live_start", { onFrame: channel });
  return () => invoke("live_stop", { id });
}

export function agentStatus(): Promise<AgentLink> {
  return invoke("agent_status");
}

/** Starts the watcher next to the window and waits up to 3 s for it to answer. */
export function startAgent(): Promise<void> {
  return invoke("start_agent");
}

/** Pauses or resumes watching; `untilMs` ends the pause by itself at that time. */
export function setPaused(paused: boolean, untilMs: number | null = null): Promise<void> {
  return invoke("set_paused", { paused, untilMs });
}

export function setSensitive(on: boolean): Promise<void> {
  return invoke("set_sensitive", { on });
}

export function setAutostart(on: boolean): Promise<void> {
  return invoke("set_autostart", { on });
}

/** Deletes everything the watcher has kept; settings stay. */
export function clearHistory(): Promise<void> {
  return invoke("clear_history");
}

export function settings(): Promise<SettingsDto> {
  return invoke("settings");
}

export function setSettings(settings: Settings): Promise<void> {
  return invoke("set_settings", { settings });
}

/** Starts a deep capture in the elevated helper; Windows asks for permission first. */
export function startCapture(minutes: number): Promise<CaptureRun> {
  return invoke("start_capture", { minutes });
}

/** The deep capture this window started, while it runs. */
export function captureRun(): Promise<CaptureRun | null> {
  return invoke("capture_run");
}

/** Stores the finished captures the watcher has not stored yet; resolves to their ids. */
export function importCaptures(): Promise<number[]> {
  return invoke("import_captures");
}

export function captures(fromMs: number, toMs: number): Promise<CaptureSummary[]> {
  return invoke("captures", { fromMs, toMs });
}

export function capture(id: number): Promise<CaptureDto | null> {
  return invoke("capture", { id });
}

/** Open health findings, those closed lately and the last full scan, read from the history. */
export function health(): Promise<HealthDto> {
  return invoke("health");
}

/** Runs a full scan in the elevated helper and waits for it; Windows asks for permission first. */
export function startScan(): Promise<ScanFinished> {
  return invoke("start_scan");
}

/** The dumps Windows kept, what each says, the space they take and Windows' blue screen reports. */
export function dumps(): Promise<DumpsDto> {
  return invoke("dumps");
}

/** Reads every dump in the elevated helper and waits for it; Windows asks for permission first. */
export function startDumpScan(): Promise<DumpScanFinished> {
  return invoke("start_dump_scan");
}

/** Deletes these dumps in the elevated helper, which checks each path again. */
export function deleteDumps(ids: number[]): Promise<DeleteFinished> {
  return invoke("delete_dumps", { ids });
}

/** Every tweak with its state and signals, the changes made so far and Windows' last restore point. */
export function tweaks(): Promise<TweaksDto> {
  return invoke("tweaks");
}

/** Applies the tweaks picked; Windows asks for permission once for those that need it. */
export function applyTweaks(picks: TweakPick[], withoutRestorePoint: boolean): Promise<TweaksApplied> {
  return invoke("apply_tweaks", { picks, withoutRestorePoint });
}

/** Puts back what one change wrote. */
export function undoTweak(id: number, withoutRestorePoint: boolean): Promise<TweaksApplied> {
  return invoke("undo_tweak", { id, withoutRestorePoint });
}

/** The Tests page: memory free now, Windows' memory diagnostics, disks, volumes and earlier runs. */
export function tests(): Promise<TestsDto> {
  return invoke("tests");
}

/** SMART or NVMe detail of one disk, read now; reading an idle HDD spins it up. */
/** A process's file, user and signature, and the threads and processes of its program, now. */
export function processFacts(pid: number, name: string): Promise<ProcessFacts> {
  return invoke("process_facts", { pid, name });
}

/** The occurrences and overload periods of `[fromMs, toMs]` that involve a program. */
export function programMarks(name: string, fromMs: number, toMs: number): Promise<ProgramMarks> {
  return invoke("program_marks", { name, fromMs, toMs });
}

/** One program's use at each ranking of the last ten minutes. */
export function programRecent(name: string): Promise<ProgramPoint[]> {
  return invoke("program_recent", { name });
}

/** The last 30 days of one SMART attribute of a disk, named as the disk counters name it. */
export function smartHistory(disk: string, attr: number): Promise<SmartPoint[]> {
  return invoke("smart_history", { disk, attr });
}

export function diskDetail(number: number, name: string): Promise<DiskDetailDto> {
  return invoke("disk_detail", { number, name });
}

/** Opens Windows' own Memory Diagnostic, which asks for permission and offers to restart. */
export function openMemoryDiagnostic(): Promise<void> {
  return invoke("open_memory_diagnostic");
}

/** Tests part of the free memory in this window; progress and the end arrive as events. */
export function startMemoryTest(minutes: number): Promise<RunningTest> {
  return invoke("start_memory_test", { minutes });
}

/** Measures one volume with a temporary file, in this window. */
export function startBenchmark(volume: string): Promise<RunningTest> {
  return invoke("start_benchmark", { volume });
}

/** Reads a whole disk in the elevated helper, from `fromMib`; Windows asks for permission first. */
export function startSurfaceScan(disk: number, fromMib: number): Promise<RunningTest> {
  return invoke("start_surface_scan", { disk, fromMib });
}

/** Checks one volume's file system in the elevated helper; Windows asks for permission first. */
export function startVolumeCheck(volume: string): Promise<RunningTest> {
  return invoke("start_volume_check", { volume });
}

/** Stops the test this window started, or the disk scan whose files `scan` names. */
export function stopTest(scan: string | null): Promise<void> {
  return invoke("stop_test", { scan });
}

/** The test this window started, while it runs. */
export function testRunning(): Promise<RunningTest | null> {
  return invoke("test_running");
}
