import { Channel, invoke } from "@tauri-apps/api/core";

import type {
  AgentLink,
  LiveMessage,
  MarksDto,
  OccurrenceDto,
  Settings,
  SettingsDto,
  TimelineDto,
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
