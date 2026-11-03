import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  AgentLink,
  CaptureDto,
  CaptureRun,
  CaptureSummary,
  DeleteFinished,
  DumpScanFinished,
  DumpsDto,
  HealthDto,
  LiveMessage,
  MarksDto,
  OccurrenceDto,
  ScanFinished,
  Settings,
  SettingsDto,
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

/** Streams the last ten minutes, then one second a second; resolves to the function that stops it. */
export async function liveStart(onMessage: (message: LiveMessage) => void): Promise<() => Promise<void>> {
  const channel = new Channel<LiveMessage>();
  channel.onmessage = onMessage;
  await invoke("live_start", { onFrame: channel });
  return () => invoke("live_stop");
}

export function agentStatus(): Promise<AgentLink> {
  return invoke("agent_status");
}

/** Starts the watcher next to the window and waits up to 3 s for it to answer. */
export function startAgent(): Promise<void> {
  return invoke("start_agent");
}

export function setPaused(paused: boolean): Promise<void> {
  return invoke("set_paused", { paused });
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
