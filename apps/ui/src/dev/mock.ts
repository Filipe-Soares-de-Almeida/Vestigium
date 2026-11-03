// Stand-in for the window's Rust side in a plain browser (npm run dev); ./fixtures/*.ts add `handlers`.
// URL: ?link=running|stopped|mismatch&paused=1&sensitive=1&history=empty&maximized=1&fail=<command>:<kind>&delay=<ms>&theme=&color=&language=&detail=
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import type { AgentLink, Settings, SettingsDto, TimelineDto, UiError } from "../lib/types";

export type Handler = (args: Record<string, unknown>) => unknown;

const query = new URLSearchParams(location.search);
const failures = new Map(
  query.getAll("fail").map((pair) => pair.split(":") as [string, UiError["kind"]]),
);

const state = {
  link: (query.get("link") ?? "running") as AgentLink["link"],
  paused: query.get("paused") === "1",
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
    state.link === "running"
      ? {
          link: "running",
          paused: state.paused,
          sensitive_until_ms: state.sensitive ? Date.now() + 1_800_000 : null,
          state: "normal",
          started_at_ms: startedAt,
          last_sample_ms: Date.now(),
          last_error: null,
          suggest_capture: query.get("suggest") === "1",
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
  set_paused: (args) => needsWatcher() ?? void (state.paused = args.paused as boolean),
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
  marks: () => ({ history: state.history, occurrences: [], episodes: [], by_test: [], events: [], gaps: [], tests: [] }),
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
