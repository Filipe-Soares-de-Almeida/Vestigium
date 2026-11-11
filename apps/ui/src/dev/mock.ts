// Stand-in for the window's Rust side in a plain browser (npm run dev); ./fixtures/*.ts add `handlers`.
// URL: ?link=running|stopped|mismatch&monitor=ok|no_reading|degraded|unverified|silent&paused=1&sensitive=1&history=empty&maximized=1&fail=<command>:<kind>&delay=<ms>&theme=&color=&language=&detail=
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import type { AgentLink, MonitorStatus, Settings, SettingsDto, TimelineDto, UiError } from "../lib/types";

export type Handler = (args: Record<string, unknown>) => unknown;

const query = new URLSearchParams(location.search);
const failures = new Map(
  query.getAll("fail").map((pair) => pair.split(":") as [string, UiError["kind"]]),
);

const state = {
  link: (query.get("link") ?? "running") as AgentLink["link"],
  paused: query.get("paused") === "1",
  pausedUntil: null as number | null,
  sensitive: query.get("sensitive") === "1",
  history: query.get("history") === "empty" ? ("empty" as const) : ("ready" as const),
  settings: {
    retention: { samples_hours: 48, minutes_days: 90, records_days: 365 },
    language: query.get("language") ?? "System",
    theme: query.get("theme") ?? "System",
    color_theme: query.get("color") ?? "Red",
    detail_level: query.get("detail") ?? "Simple",
  } as Settings,
  autostart: true,
  maximized: query.get("maximized") === "1",
};

const startedAt = Date.now();
const monitorMode = query.get("monitor") ?? "ok";

/** The watcher's account of its own watching, for the `monitor` URL parameter. */
function monitorStatus(): MonitorStatus {
  const now = Date.now();
  const watching: MonitorStatus = {
    state: "watching",
    failing: [],
    no_reading_since_ms: null,
    resumed_ms: startedAt,
    health_attempt_ms: now - 60_000,
    health_ok_ms: now - 60_000,
    health_unread: [],
  };
  if (monitorMode === "no_reading") {
    return { ...watching, state: "no_reading", no_reading_since_ms: now - 82 * 60_000 };
  }
  if (monitorMode === "degraded") {
    return { ...watching, state: "degraded", failing: [{ source: "counters", since_ms: now - 82 * 60_000, error: "PdhCollectQueryData failed: 0xC0000BBD" }] };
  }
  if (monitorMode === "unverified") {
    return { ...watching, health_attempt_ms: now - 60_000, health_ok_ms: now - 4 * 3_600_000, health_unread: ["TwoAntivirus"] };
  }
  return watching;
}
const delayMs = Number(query.get("delay") ?? 0);

function refuse(kind: UiError["kind"]): Promise<never> {
  return Promise.reject({ kind, message: `mock: ${kind}` } satisfies UiError);
}

/** Commands that need a running watcher fail as the real ones do when it is not. */
function needsWatcher(): Promise<never> | null {
  if (state.link === "stopped") return refuse("agent_stopped");
  if (state.link === "mismatch") return refuse("mismatch");
  return null;
}

const base: Record<string, Handler> = {
  agent_status: () =>
    state.link === "running" && monitorMode === "silent"
      ? { link: "silent", seconds: 3 }
      : state.link === "running"
      ? {
          link: "running",
          paused: state.paused,
          paused_until_ms: state.paused ? state.pausedUntil : null,
          sensitive_until_ms: state.sensitive ? Date.now() + 1_800_000 : null,
          state: "normal",
          started_at_ms: startedAt,
          last_sample_ms: monitorMode === "no_reading" ? Date.now() - 82 * 60_000 : Date.now(),
          last_error: monitorMode === "no_reading" ? "the performance counters could not be opened" : null,
          suggest_capture: query.get("suggest") === "1",
          cpu_pct: 0.1,
          suspended: false,
          monitor: monitorStatus(),
        }
      : { link: state.link },
  settings: () => {
    if (state.link === "mismatch") return refuse("mismatch");
    const editable = state.link === "running";
    return {
      settings: state.settings,
      autostart: editable ? state.autostart : null,
      editable,
    } satisfies SettingsDto;
  },
  start_agent: () =>
    new Promise((resolve) =>
      setTimeout(() => {
        state.link = "running";
        resolve(null);
      }, 800),
    ),
  set_paused: (args) =>
    needsWatcher() ??
    void ((state.paused = args.paused as boolean), (state.pausedUntil = (args.untilMs as number | null) ?? null)),
  set_sensitive: (args) => needsWatcher() ?? void (state.sensitive = args.on as boolean),
  set_autostart: (args) => needsWatcher() ?? void (state.autostart = args.on as boolean),
  set_settings: (args) => needsWatcher() ?? void (state.settings = args.settings as Settings),
  clear_history: () => needsWatcher() ?? void (state.history = "empty"),
  timeline: (args) =>
    ({
      history: state.history,
      resolution: "minutes",
      step_ms: 60_000,
      t: [Math.floor((args.fromMs as number) / 1000)],
      cpu: [null],
      mem_used_pct: [null],
      disk_active_max: [null],
      disk_latency_max: [null],
      gpu: [null],
      probe_lag: [null],
      state: ["suspended"],
    }) satisfies TimelineDto,
  marks: () => ({ history: state.history, occurrences: [], episodes: [], by_test: [], gaps: [], tests: [] }),
  occurrence: () => null,
  live_start: () => needsWatcher(),
  live_stop: () => null,
  "plugin:window|is_maximized": () => state.maximized,
  "plugin:window|toggle_maximize": () => void (state.maximized = !state.maximized),
  "plugin:window|minimize": () => null,
  "plugin:window|close": () => null,
  "plugin:window|start_dragging": () => null,
  "plugin:window|internal_toggle_maximize": () => void (state.maximized = !state.maximized),
};

const fixtures = import.meta.glob<{ handlers: Record<string, Handler> }>("./fixtures/*.ts", {
  eager: true,
});
const handlers: Record<string, Handler> = Object.assign(
  base,
  ...Object.values(fixtures).map((fixture) => fixture.handlers),
);

mockWindows("main");
mockIPC(
  (command, payload) => {
    console.debug(`[mock] ${command}`);
    const kind = failures.get(command);
    if (kind) return refuse(kind);
    const handler = handlers[command];
    if (!handler) return refuse("failed");
    const answer = () => handler((payload ?? {}) as Record<string, unknown>);
    return delayMs ? new Promise((resolve) => setTimeout(resolve, delayMs)).then(answer) : answer();
  },
  { shouldMockEvents: true },
);
