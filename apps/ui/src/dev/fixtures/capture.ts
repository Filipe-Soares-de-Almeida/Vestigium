// Deep captures in the browser: #/capture/1 names a driver and an app, 2 has no frames, 3 lost events,
// 4 found nothing, 5 has an app with 8 % late frames beside a steady one, 6 lost its frames after 3 of
// 5 minutes but still names a driver, 7 lost them after 3 minutes and found nothing, 8 measured stalls
// but no driver went over the limits and 9 could not measure drivers. `?capture-secs=<n>` makes a started capture last n seconds instead of its minutes,
// `?capture-fail=1` makes it end with the helper failing and `?capture-left=<n>` opens the window with a
// capture that has n seconds left. `?capture-auto=1` presses the start button once the page is up, and
// `?capture-ask=<ms>` keeps the permission prompt open that long and `?capture-none=1` starts with no
// stored capture. `?capture-else=<n>` opens the window with a capture another window started that ends
// in n seconds, with `?capture-else-start=0` when only its existence is known.
import { emit } from "@tauri-apps/api/event";

import type {
  CaptureDto,
  CaptureRun,
  DriverLatency,
  FrameHitch,
  RoutineStats,
  Suspect,
} from "../../lib/types";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const BOUNDS = [1, 2, 3, 4, 5, 10, 25, 50, 100, 250, 500, 1000];

/** The newest stored capture started three hours ago. */
const START = Math.floor((Date.now() - 3 * 3_600_000) / 60_000) * 60_000;
const LENGTH = 5 * 60_000;

function stats(count: number, totalUs: number, maxUs: number, slow: number): RoutineStats {
  const buckets = new Array(BOUNDS.length + 1).fill(0);
  const fast = count - slow;
  buckets[5] = Math.round(fast * 0.6);
  buckets[6] = fast - buckets[5];
  const index = BOUNDS.findIndex((b) => maxUs < b);
  buckets[9] = Math.max(slow - 1, 0);
  buckets[index === -1 ? BOUNDS.length : index] += slow > 0 ? 1 : 0;
  return { count, total_us: totalUs, max_us: maxUs, buckets };
}

const empty: RoutineStats = { count: 0, total_us: 0, max_us: 0, buckets: new Array(13).fill(0) };

/** The drivers of a capture; `slow` is how many calls of the graphics driver passed 100 us. */
function drivers(slow: number): DriverLatency[] {
  return [
    { module: "storport.sys", dpc: stats(219_000, 974_000, 57, 0), isr: stats(224_000, 829_000, 247, 6) },
    { module: "gfxdriver.sys", dpc: stats(31_000, 182_000, 1_240, slow), isr: empty },
    { module: "dxgkrnl.sys", dpc: stats(23_800, 176_000, 97, 0), isr: stats(25_000, 641_000, 256, 8) },
    { module: "ntoskrnl.exe", dpc: stats(12_600, 76_000, 115, 1), isr: empty },
    { module: "tcpip.sys", dpc: stats(415, 10_400, 100, 1), isr: empty },
  ];
}

/** Drivers that all stayed inside the recommended limits. */
const CALM_DRIVERS: DriverLatency[] = [
  { module: "storport.sys", dpc: stats(21_000, 94_000, 41, 0), isr: stats(22_000, 83_000, 18, 0) },
  { module: "gfxdriver.sys", dpc: stats(9_000, 52_000, 63, 0), isr: empty },
  { module: "ntoskrnl.exe", dpc: stats(12_600, 76_000, 88, 0), isr: empty },
];

const HITCHES: FrameHitch[] = [62_000, 141_500, 141_520, 203_000, 244_000].map((offset, i) => ({
  ts_ms: START + offset,
  app: "Starfall.exe",
  pid: 9124,
  frame_ms: [166, 48, 41, 92, 35][i],
  median_ms: 6.9,
}));

function routine(module: string, us: number, maxUs: number, count: number, slow: number, corePct: number) {
  return {
    Routine: {
      module,
      kind: "Dpc" as const,
      count,
      max_us: maxUs,
      at_least_100us: slow,
      at_least_1ms: us >= 1000 ? 1 : 0,
      core_pct: corePct,
    },
  };
}

function findings(slow: number): Suspect[] {
  return [
    {
      cause: { DriverModule: { module: "gfxdriver.sys", intermediary: false } },
      confidence: "High",
      evidence: [routine("gfxdriver.sys", 1_240, 1_240, 31_000, slow, 0.06)],
      next_steps: ["UpdateDriver"],
    },
    {
      cause: { Frames: { app: "Starfall.exe" } },
      confidence: "Medium",
      evidence: [
        { Frames: { app: "Starfall.exe", frames: 18_000, hitches: 14, worst_ms: 166, median_ms: 6.9 } },
        ...HITCHES.slice(0, 3).map((h) => ({ Hitch: { ts_ms: h.ts_ms, app: h.app, frame_ms: h.frame_ms } })),
      ],
      next_steps: [],
    },
    {
      cause: { DriverModule: { module: "storport.sys", intermediary: true } },
      confidence: "Low",
      evidence: [routine("storport.sys", 247, 247, 443_000, 6, 0.6)],
      next_steps: [],
    },
  ];
}

function build(id: number, start: number, slow = 380): Omit<CaptureDto, "kept"> {
  const FINDINGS = findings(slow);
  const base: Omit<CaptureDto, "kept"> = {
    id,
    start_ms: start,
    end_ms: start + LENGTH,
    kernel: "Ok",
    frames: "Ok",
    lost_events: 0,
    drivers: drivers(slow),
    longest: [
      { ts_ms: start + 141_480, core: 0, module: "gfxdriver.sys", kind: "Dpc", us: 1_240 },
      { ts_ms: start + 62_010, core: 0, module: "gfxdriver.sys", kind: "Dpc", us: 1_090 },
      { ts_ms: start + 203_000, core: 0, module: "gfxdriver.sys", kind: "Dpc", us: 1_010 },
      { ts_ms: start + 90_300, core: 0, module: "dxgkrnl.sys", kind: "Isr", us: 256 },
    ],
    apps: [{ name: "Starfall.exe", pid: 9124, frames: 18_000, median_ms: 6.9, p99_ms: 13.1, worst_ms: 166, hitches: 14 }],
    hitches: HITCHES.map((h) => ({ ...h, ts_ms: h.ts_ms - START + start })),
    hiccups: [
      { ts_ms: start + 62_004, lag_ms: 12.4 },
      { ts_ms: start + 141_490, lag_ms: 40.2 },
      { ts_ms: start + 203_010, lag_ms: 18.7 },
    ],
    findings: FINDINGS,
  };
  if (id === 4) {
    return {
      ...base,
      drivers: CALM_DRIVERS,
      longest: [],
      apps: [{ name: "Starfall.exe", pid: 9124, frames: 18_000, median_ms: 6.9, p99_ms: 9.4, worst_ms: 14, hitches: 0 }],
      hitches: [],
      hiccups: [],
      findings: [],
    };
  }
  if (id === 5) {
    return {
      ...base,
      drivers: CALM_DRIVERS,
      longest: [],
      apps: [
        { name: "Starfall.exe", pid: 9124, frames: 18_000, median_ms: 6.9, p99_ms: 41, worst_ms: 166, hitches: 1_440 },
        { name: "Editor.exe", pid: 3380, frames: 9_200, median_ms: 16.6, p99_ms: 22, worst_ms: 38, hitches: 0 },
      ],
      hitches: [],
      hiccups: [],
      findings: [],
    };
  }
  const partial = { Partial: { covered_secs: 180, reason: "PresentMon exited with code 1 after 180 s" } };
  if (id === 6) {
    return { ...base, frames: partial, apps: [{ ...base.apps[0], frames: 10_800 }], hitches: base.hitches.slice(0, 3) };
  }
  if (id === 7) {
    return { ...base, ...quiet(), frames: partial, apps: [{ name: "Editor.exe", pid: 3380, frames: 5_400, median_ms: 16.6, p99_ms: 22, worst_ms: 36, hitches: 0 }] };
  }
  if (id === 8) {
    return { ...base, ...quiet(), apps: [{ name: "Editor.exe", pid: 3380, frames: 9_200, median_ms: 16.6, p99_ms: 24, worst_ms: 38, hitches: 0 }], hiccups: base.hiccups };
  }
  if (id === 9) {
    return {
      ...base,
      ...quiet(),
      kernel: { Unavailable: "ETW session could not start (error 183, name already in use)" },
      drivers: [],
      apps: [{ name: "Editor.exe", pid: 3380, frames: 9_200, median_ms: 16.6, p99_ms: 22, worst_ms: 34, hitches: 0 }],
    };
  }
  if (id === 2) {
    return {
      ...base,
      frames: { Unavailable: "PresentMon saw presents but finished none" },
      apps: [],
      hitches: [],
      findings: FINDINGS.filter((s) => !("Frames" in (s.cause as object))),
    };
  }
  if (id === 3) {
    return {
      ...base,
      lost_events: 1_812,
      findings: FINDINGS.map((s) => ({
        ...s,
        confidence: s.confidence === "High" ? "Medium" : s.confidence,
        evidence: [...s.evidence, { CaptureLoss: { lost_events: 1_812 } }],
      })),
    };
  }
  return base;
}

/** The capture with every list of what stood out emptied, and its drivers inside the limits. */
function quiet(): Pick<CaptureDto, "drivers" | "longest" | "hitches" | "hiccups" | "findings"> {
  return { drivers: CALM_DRIVERS, longest: [], hitches: [], hiccups: [], findings: [] };
}

export function capture(id: number, start: number, slow = 380): CaptureDto {
  const dto = build(id, start, slow);
  return { ...dto, kept: { calls: dto.longest.length, hitches: dto.hitches.length } };
}

const STORED = new Map(
  (query.get("capture-none") === "1" ? [] : [1, 2, 3, 4, 5, 6, 7, 8, 9]).map((id) => [id, capture(id, START - (id - 1) * 3_600_000, id === 1 ? 12 : 380)]),
);
const left = Number(query.get("capture-left") ?? 0);
const elsewhere = Number(query.get("capture-else") ?? 0);
let run: CaptureRun | null = left
  ? { started_ms: Date.now() - 48_000, ends_ms: Date.now() + left * 1000, own: true }
  : elsewhere
    ? { started_ms: query.get("capture-else-start") === "0" ? null : Date.now() - 48_000, ends_ms: null, own: false }
    : null;
/** What the watcher stored since the window last asked. */
let stored: number[] = [];
if (elsewhere) {
  setTimeout(() => {
    const id = Math.max(0, ...STORED.keys()) + 1;
    STORED.set(id, capture(id, Date.now() - elsewhere * 1000, 4));
    stored = [id];
    run = null;
  }, elsewhere * 1000);
}

let ending: ReturnType<typeof setTimeout> | undefined;

export const handlers: Record<string, Handler> = {
  start_capture: async (args) => {
    const minutes = Number(args.minutes);
    if (run) throw { kind: "capture_running", message: "a deep capture is already running" };
    await new Promise((resolve) => setTimeout(resolve, Number(query.get("capture-ask") ?? 0)));
    const seconds = Number(query.get("capture-secs") ?? minutes * 60);
    const now = Date.now();
    run = { started_ms: now, ends_ms: now + seconds * 1000, own: true };
    ending = setTimeout(() => {
      const id = Math.max(0, ...STORED.keys()) + 1;
      run = null;
      if (query.get("capture-late") === "1") {
        void emit("capture-finished", { ids: [], exit_code: null, ended: "TimedOut" });
        return;
      }
      if (query.get("capture-fail") === "1") {
        void emit("capture-finished", { ids: [], exit_code: 1, ended: "Done" });
        return;
      }
      STORED.set(id, capture(id, now, 4));
      void emit("capture-finished", { ids: [id], exit_code: 0, ended: "Done" });
    }, seconds * 1000);
    return run;
  },
  capture_run: () => run,
  cancel_capture: () => {
    clearTimeout(ending);
    setTimeout(() => {
      run = null;
      void emit("capture-finished", { ids: [], exit_code: 5, ended: "Cancelled" });
    }, 600);
    return null;
  },
  import_captures: () => stored.splice(0),
  captures: (args) =>
    [...STORED.values()]
      .filter((c) => c.start_ms <= (args.toMs as number) && c.end_ms >= (args.fromMs as number))
      .map(({ id, start_ms, end_ms }) => ({ id, start_ms, end_ms }))
      .sort((a, b) => a.start_ms - b.start_ms),
  capture: (args) => STORED.get(args.id as number) ?? null,
};

if (query.get("capture-auto") === "1") {
  setTimeout(() => document.querySelector<HTMLButtonElement>(".start .btn.accent")?.click(), 1500);
}
