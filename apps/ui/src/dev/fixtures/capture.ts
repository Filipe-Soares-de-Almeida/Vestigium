// Deep captures in the browser: #/capture/1 names a driver and an app, 2 has no frames, 3 lost events and
// 4 found nothing. `?capture-secs=<n>` makes a started capture last n seconds instead of its minutes,
// `?capture-fail=1` makes it end with the helper failing and `?capture-left=<n>` opens the window with a
// capture that has n seconds left. `?capture-auto=1` presses the start button once the page is up, and
// `?capture-ask=<ms>` keeps the permission prompt open that long and `?capture-none=1` starts with no
// stored capture.
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

function capture(id: number, start: number, slow = 380): CaptureDto {
  const FINDINGS = findings(slow);
  const base: CaptureDto = {
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

const STORED = new Map(
  (query.get("capture-none") === "1" ? [] : [1, 2, 3, 4]).map((id) => [id, capture(id, START - (id - 1) * 3_600_000, id === 1 ? 12 : 380)]),
);
const left = Number(query.get("capture-left") ?? 0);
let run: CaptureRun | null = left ? { started_ms: Date.now() - 48_000, ends_ms: Date.now() + left * 1000 } : null;

export const handlers: Record<string, Handler> = {
  start_capture: async (args) => {
    const minutes = Number(args.minutes);
    await new Promise((resolve) => setTimeout(resolve, Number(query.get("capture-ask") ?? 0)));
    const seconds = Number(query.get("capture-secs") ?? minutes * 60);
    const now = Date.now();
    run = { started_ms: now, ends_ms: now + seconds * 1000 };
    setTimeout(() => {
      const id = Math.max(0, ...STORED.keys()) + 1;
      run = null;
      if (query.get("capture-fail") === "1") {
        void emit("capture-finished", { ids: [], exit_code: 1 });
        return;
      }
      STORED.set(id, capture(id, now, 4));
      void emit("capture-finished", { ids: [id], exit_code: 0 });
    }, seconds * 1000);
    return run;
  },
  capture_run: () => run,
  import_captures: () => [],
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
