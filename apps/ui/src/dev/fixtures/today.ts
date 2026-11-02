// Synthetic history for Today: ?day=full (default), quiet (no stalls) or empty (nothing watched today);
// ?history=empty is the first use, ?fall=1 adds a fall at 7:12. Nothing here was measured on a real PC.
import type {
  Episode,
  Gap,
  MarksDto,
  OccurrenceKind,
  OccurrenceSummary,
  PcState,
  Severity,
  TimelineDto,
} from "../../lib/types";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const shape = query.get("day") ?? "full";
const firstUse = query.get("history") === "empty";

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const BUSY_ABOVE = 38;

function midnight(offset: number): number {
  const date = new Date();
  date.setHours(0, 0, 0, 0);
  date.setDate(date.getDate() + offset);
  return date.getTime();
}

const today = midnight(0);
const at = (h: number, m = 0, s = 0) => today + h * HOUR + m * MINUTE + s * SECOND;

/** The same pseudo-random number for the same minute and channel, on every reload. */
function noise(minute: number, channel: number): number {
  const v = Math.sin(minute * 12.9898 + channel * 78.233) * 43758.5453;
  return v - Math.floor(v);
}

const gaps: Gap[] =
  shape === "empty"
    ? [{ start_ms: today, end_ms: Date.now(), reason: "Stopped" }]
    : [
        { start_ms: midnight(-1) + 23 * HOUR, end_ms: midnight(-1) + 23 * HOUR + 30 * MINUTE, reason: "Stopped" },
        { start_ms: at(1, 12), end_ms: at(8, 3), reason: "Suspended" },
        { start_ms: at(12, 30), end_ms: at(12, 44), reason: "Paused" },
      ];

const episodes: Episode[] =
  shape === "full"
    ? [
        {
          kind: "CpuSaturated",
          start_ms: at(10, 2),
          end_ms: at(10, 3, 15),
          peak: 99,
          culprit: { pid: 4120, name: "Builder.exe", value: 88 },
          truncated: false,
        },
        {
          kind: { DpcIsrHint: { core: 0 } },
          start_ms: at(11, 39),
          end_ms: at(11, 41),
          peak: 21,
          culprit: null,
          truncated: false,
        },
        {
          kind: { DiskBusy: { disk: "0 C:" } },
          start_ms: at(14, 31),
          end_ms: at(14, 33),
          peak: 100,
          culprit: { pid: 2208, name: "Indexer.exe", value: 61 },
          truncated: false,
        },
        {
          kind: "MemoryHigh",
          start_ms: at(16, 10),
          end_ms: at(16, 24),
          peak: 95,
          culprit: { pid: 9031, name: "Browser.exe", value: 5.1e9 },
          truncated: false,
        },
      ]
    : [];

function occurrence(
  id: number,
  kind: OccurrenceKind,
  start_ms: number,
  seconds: number,
  severity: Severity,
  cause: OccurrenceSummary["cause"],
): OccurrenceSummary {
  return { id, kind, start_ms, end_ms: start_ms + seconds * SECOND, severity, cause };
}

const todays: OccurrenceSummary[] =
  shape === "full"
    ? [
        occurrence(31, "AppHang", at(9, 14, 7), 12, "Severe", { App: { name: "Chatter" } }),
        occurrence(32, "Stutter", at(11, 40, 2), 10, "Light", { Driver: { core: 0 } }),
        occurrence(33, "Freeze", at(13, 5, 44), 1.9, "Medium", "GpuDriver"),
        occurrence(34, "Freeze", at(14, 32, 18), 6.8, "Severe", { Storage: { disk: "0 C:" } }),
        occurrence(35, "Stutter", at(20, 41, 2), 10, "Light", { Cpu: { processes: ["Game.exe"] } }),
        occurrence(36, "Freeze", at(22, 10, 5), 2.4, "Medium", { Memory: { process: "Browser.exe" } }),
        ...(query.get("fall") === "1"
          ? [occurrence(8, "PostMortem", at(7, 12, 25), 5, "Severe", { BlueScreen: { code: 0x50 } })]
          : []),
      ]
    : [];

/** Freezes of the 13 days before today, a few a day. */
const earlier: OccurrenceSummary[] = [1, 0, 2, 0, 0, 3, 1, 0, 1, 0, 2, 0, 1].flatMap((count, day) =>
  Array.from({ length: count }, (_, i) =>
    occurrence(
      day * 3 + i + 1,
      i % 2 ? "Stutter" : "Freeze",
      midnight(day - 13) + 15 * HOUR + i * 37 * MINUTE,
      i % 2 ? 4 : 1.4 + day * 0.3,
      i === 2 ? "Severe" : i ? "Light" : "Medium",
      i % 2 ? "Unclear" : { Storage: { disk: "0 C:" } },
    ),
  ),
);

const occurrences = [...earlier, ...todays];

interface Point {
  cpu: number;
  mem: number;
  disk: number;
  gpu: number;
  lag: number;
}

/** The reading of the minute that holds `ms`, or `null` where nothing was watched. */
function reading(ms: number): Point | null {
  if (firstUse || ms > Date.now()) return null;
  if (gaps.some((g) => ms >= g.start_ms && ms < g.end_ms)) return null;
  const minute = Math.floor((ms - today) / MINUTE);
  const m = ((minute % 1440) + 1440) % 1440;
  const between = (a: number, b: number) => m >= a && m < b;
  const gaming = between(0, 72) || between(18 * 60 + 40, 21 * 60 + 30);
  let cpu = 7 + noise(minute, 1) * 9 + (noise(minute, 2) < 0.04 ? 25 * noise(minute, 3) : 0);
  let mem = 46 + (m / 1440) * 10 + noise(minute, 4) * 3;
  let disk = 2 + noise(minute, 5) * 6 + (noise(minute, 6) < 0.05 ? 40 * noise(minute, 7) : 0);
  let gpu = 2 + noise(minute, 8) * 4;
  if (gaming) {
    cpu = 34 + noise(minute, 1) * 22;
    mem = 68 + noise(minute, 4) * 6;
    gpu = 84 + noise(minute, 8) * 15;
  }
  if (shape === "full") {
    if (between(10 * 60 + 2, 10 * 60 + 4)) cpu = 97 + 3 * noise(minute, 9);
    if (between(13 * 60 + 5, 13 * 60 + 6)) gpu = 100;
    if (between(14 * 60 + 20, 14 * 60 + 41)) disk = 45 + 30 * noise(minute, 10);
    if (between(14 * 60 + 31, 14 * 60 + 33)) disk = 100;
    if (between(16 * 60 + 2, 16 * 60 + 10)) mem = 78 + (m - (16 * 60 + 2)) * 1.6;
    if (between(16 * 60 + 10, 16 * 60 + 24)) mem = 92 + 3 * noise(minute, 11);
  }
  const clamp = (v: number) => Math.min(100, Math.max(0, v));
  return { cpu: clamp(cpu), mem: clamp(mem), disk: clamp(disk), gpu: clamp(gpu), lag: 1 + noise(minute, 12) };
}

function load(p: Point): number {
  return Math.max(p.cpu, p.disk, p.gpu * 0.55, p.mem - 35);
}

const pressure = episodes.filter((e) => typeof e.kind === "string" || "DiskBusy" in e.kind);

/** As the window's Rust side answers: seconds up to 6 h, minutes above, peaks kept per point. */
function timeline(args: Record<string, unknown>): TimelineDto {
  const from = args.fromMs as number;
  const to = args.toMs as number;
  const native = to - from <= 6 * HOUR ? SECOND : MINUTE;
  const start = from - (((from % native) + native) % native);
  const points = Math.min(10_000, Math.max(1, args.maxPoints as number));
  const step = Math.max(1, Math.ceil(Math.ceil(Math.max(1, to - start) / points) / native)) * native;
  const columns: Omit<TimelineDto, "history" | "resolution" | "step_ms"> = {
    t: [],
    cpu: [],
    mem_used_pct: [],
    disk_active_max: [],
    disk_latency_max: [],
    gpu: [],
    probe_lag: [],
    state: [],
  };
  if (!firstUse) {
    for (let b = start; b < to; b += step) {
      let peak: Point | null = null;
      for (let ms = b; ms < b + step; ms += Math.min(step, MINUTE)) {
        const r = reading(ms);
        if (!r) continue;
        peak = peak
          ? {
              cpu: Math.max(peak.cpu, r.cpu),
              mem: Math.max(peak.mem, r.mem),
              disk: Math.max(peak.disk, r.disk),
              gpu: Math.max(peak.gpu, r.gpu),
              lag: Math.max(peak.lag, r.lag),
            }
          : r;
      }
      const inEpisode = pressure.some((e) => e.start_ms < b + step && e.end_ms >= b);
      const state: PcState = !peak
        ? "suspended"
        : inEpisode
          ? "pressure"
          : load(peak) > BUSY_ABOVE
            ? "busy"
            : "calm";
      columns.t.push(b / 1000);
      columns.cpu.push(peak?.cpu ?? null);
      columns.mem_used_pct.push(peak?.mem ?? null);
      columns.disk_active_max.push(peak?.disk ?? null);
      columns.disk_latency_max.push(peak ? 1 + peak.disk / 20 : null);
      columns.gpu.push(peak?.gpu ?? null);
      columns.probe_lag.push(peak?.lag ?? null);
      columns.state.push(state);
    }
  }
  return {
    history: firstUse ? "empty" : "ready",
    resolution: native === SECOND ? "seconds" : "minutes",
    step_ms: step,
    ...columns,
  };
}

function marks(args: Record<string, unknown>): MarksDto {
  const from = args.fromMs as number;
  const to = args.toMs as number;
  const now = Date.now();
  if (firstUse) return { history: "empty", occurrences: [], episodes: [], events: [], gaps: [] };
  return {
    history: "ready",
    occurrences: occurrences.filter((o) => o.start_ms >= from && o.start_ms < to && o.start_ms < now),
    episodes: episodes.filter((e) => e.end_ms > from && e.start_ms < to && e.start_ms < now),
    events: [],
    gaps: gaps.filter((g) => g.end_ms > from && g.start_ms < to && g.start_ms < now),
  };
}

export const handlers: Record<string, Handler> = { timeline, marks };
