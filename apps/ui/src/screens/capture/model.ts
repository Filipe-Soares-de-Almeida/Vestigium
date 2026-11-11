import type {
  AppFrames,
  CaptureDto,
  CaptureSummary,
  RoutineStats,
  Suspect,
} from "../../lib/types";

// What a deep capture says, worked out from its typed numbers: the driver ranking, the apps with late
// frames and the moments where a stall, a long driver call and a late frame fell together.

/** Recommended longest DPC and ISR, in microseconds. */
export const DPC_LIMIT_US = 100;
export const ISR_LIMIT_US = 25;
/** A driver call this long, in microseconds, is felt on its own. */
export const FELT_US = 1000;
/** Marks this close, in milliseconds, belong to one moment. */
export const NEAR_MS = 50;

const BOUNDS_US = [1, 2, 3, 4, 5, 10, 25, 50, 100, 250, 500, 1000];
const INTERMEDIARIES = ["wdf01000.sys", "ndis.sys", "storport.sys", "ataport.sys", "dxgkrnl.sys"];
const KERNEL_CORE = ["ntoskrnl.exe", "hal.dll"];
const UNKNOWN_MODULE = "<unknown>";

export const isIntermediary = (module: string) => INTERMEDIARIES.includes(module.toLowerCase());
export const isKernelCore = (module: string) => KERNEL_CORE.includes(module.toLowerCase());
const nameable = (module: string) => !isKernelCore(module) && module !== UNKNOWN_MODULE;

/** Calls in buckets that start at or above `us`; exact when `us` is a bucket bound. */
function atLeast(stats: RoutineStats, us: number): number {
  return stats.buckets.reduce((sum, n, i) => ((i === 0 ? 0 : BOUNDS_US[i - 1]) >= us ? sum + n : sum), 0);
}

export interface DriverRow {
  module: string;
  kernel: boolean;
  intermediary: boolean;
  /** A driver the capture names itself, not a layer that serves others. */
  suspect: boolean;
  finding: Suspect | null;
  dpc: RoutineStats;
  isr: RoutineStats;
  calls: number;
  totalUs: number;
  maxUs: number;
  maxKind: "Dpc" | "Isr";
  /** The longest call as a multiple of its recommended limit. */
  times: number;
  /** Calls over the recommended limit: DPC of 100 µs and ISR of 25 µs. */
  over: number;
}

function findingModule(s: Suspect): string | null {
  return typeof s.cause !== "string" && "DriverModule" in s.cause ? s.cause.DriverModule.module : null;
}

/**
 * Drivers ordered by what the conclusion rests on: the ones the capture names, in its own order,
 * then the rest by how far their longest call went past the recommended limit. Total time never
 * ranks them, so a layer that serves other drivers does not look like the culprit; the kernel and
 * unknown addresses go last.
 */
export function driverRows(dto: CaptureDto): DriverRow[] {
  const rows = dto.drivers.map((d): DriverRow => {
    const finding = dto.findings.find((s) => findingModule(s)?.toLowerCase() === d.module.toLowerCase()) ?? null;
    const intermediary = isIntermediary(d.module);
    const maxKind = d.dpc.max_us / DPC_LIMIT_US >= d.isr.max_us / ISR_LIMIT_US ? "Dpc" : "Isr";
    return {
      module: d.module,
      kernel: isKernelCore(d.module),
      intermediary,
      suspect: finding !== null && !intermediary,
      finding,
      dpc: d.dpc,
      isr: d.isr,
      calls: d.dpc.count + d.isr.count,
      totalUs: d.dpc.total_us + d.isr.total_us,
      maxUs: Math.max(d.dpc.max_us, d.isr.max_us),
      maxKind,
      times: Math.max(d.dpc.max_us / DPC_LIMIT_US, d.isr.max_us / ISR_LIMIT_US),
      over: atLeast(d.dpc, DPC_LIMIT_US) + atLeast(d.isr, ISR_LIMIT_US),
    };
  });
  const group = (r: DriverRow) => (r.finding ? 0 : nameable(r.module) ? 1 : 2);
  const place = (r: DriverRow) => (r.finding ? dto.findings.indexOf(r.finding) : 0);
  return rows.sort(
    (a, b) =>
      group(a) - group(b) ||
      place(a) - place(b) ||
      b.times - a.times ||
      b.over - a.over ||
      a.module.localeCompare(b.module),
  );
}

export type Conclusion =
  | { kind: "driver" | "intermediary"; finding: Suspect; row: DriverRow }
  | { kind: "app"; finding: Suspect; app: AppFrames | null }
  | { kind: "none" };

/** What the capture's first finding is, with the numbers behind it. */
export function conclusionOf(dto: CaptureDto, rows: DriverRow[]): Conclusion {
  const finding = dto.findings[0];
  if (!finding) return { kind: "none" };
  const module = findingModule(finding);
  if (module !== null) {
    const row = rows.find((r) => r.module.toLowerCase() === module.toLowerCase());
    if (row) return { kind: row.intermediary ? "intermediary" : "driver", finding, row };
  }
  if (typeof finding.cause !== "string" && "Frames" in finding.cause) {
    const name = finding.cause.Frames.app;
    return { kind: "app", finding, app: dto.apps.find((a) => a.name === name) ?? null };
  }
  return { kind: "none" };
}

export interface Mark {
  lane: 0 | 1 | 2;
  /** Milliseconds from the start of the capture. */
  at: number;
  ts_ms: number;
  /** Stall in ms, call in µs or frame in ms. */
  value: number;
  /** The driver or app the mark belongs to; empty for a stall. */
  who: string;
  /** The moment the mark belongs to, or -1 when it stands alone. */
  moment: number;
}

export interface Alignment {
  marks: Mark[];
  /** Milliseconds from the start of each moment where marks of two or more lanes fell together. */
  moments: number[];
  stalls: number;
  stallsWithCall: number;
  stallsWithFrame: number;
  /** Frames' own hitches that came with a long call or a stall. */
  frameHitches: number;
  maxStallMs: number;
  minStallMs: number;
}

/** Puts stalls, long driver calls and late frames on the capture's clock and finds where they met. */
export function alignment(dto: CaptureDto): Alignment {
  const rel = (ts: number) => ts - dto.start_ms;
  const marks: Mark[] = [
    ...dto.hiccups.map((h): Mark => ({ lane: 0, at: rel(h.ts_ms), ts_ms: h.ts_ms, value: h.lag_ms, who: "", moment: -1 })),
    ...dto.longest
      .filter((l) => nameable(l.module) && l.us >= FELT_US)
      .map((l): Mark => ({ lane: 1, at: rel(l.ts_ms), ts_ms: l.ts_ms, value: l.us, who: l.module, moment: -1 })),
    ...dto.hitches.map((h): Mark => ({ lane: 2, at: rel(h.ts_ms), ts_ms: h.ts_ms, value: h.frame_ms, who: h.app, moment: -1 })),
  ].sort((a, b) => a.at - b.at || a.lane - b.lane);

  const groups: Mark[][] = [];
  for (const mark of marks) {
    const last = groups.at(-1);
    if (last && mark.at - last[last.length - 1].at <= NEAR_MS) last.push(mark);
    else groups.push([mark]);
  }
  const moments: number[] = [];
  for (const group of groups) {
    if (new Set(group.map((m) => m.lane)).size < 2) continue;
    for (const mark of group) mark.moment = moments.length;
    moments.push(group[0].at);
  }

  const near = (mark: Mark, lane: number) => marks.some((m) => m.lane === lane && Math.abs(m.at - mark.at) <= NEAR_MS);
  const stalls = marks.filter((m) => m.lane === 0);
  const lags = stalls.map((m) => m.value);
  return {
    marks,
    moments,
    stalls: stalls.length,
    stallsWithCall: stalls.filter((m) => near(m, 1)).length,
    stallsWithFrame: stalls.filter((m) => near(m, 2)).length,
    frameHitches: marks.filter((m) => m.lane === 2 && (near(m, 1) || near(m, 0))).length,
    maxStallMs: Math.max(0, ...lags),
    minStallMs: lags.length ? Math.min(...lags) : 0,
  };
}

/** Late frames of one app that came together with a long driver call, when the capture kept all of them. */
export function hitchesWithCall(dto: CaptureDto, app: AppFrames): number | null {
  const own = dto.hitches.filter((h) => h.app === app.name && h.pid === app.pid);
  if (own.length < app.hitches) return null;
  const calls = dto.longest.filter((l) => nameable(l.module) && l.us >= FELT_US);
  return own.filter((h) => calls.some((l) => Math.abs(l.ts_ms - h.ts_ms) <= NEAR_MS)).length;
}

/** Apps whose frames ran late, most first. */
export const lateApps = (dto: CaptureDto): AppFrames[] => dto.apps.filter((a) => a.hitches > 0);

/** Above this share of late frames an app is marked, never named: one that draws only on demand looks late when idle. */
export const OFTEN_PCT = 5;

/** The app's late frames as a percentage of its frames. */
export const lateShare = (app: AppFrames): number => (app.frames === 0 ? 0 : (app.hitches / app.frames) * 100);

/** An app with more late frames than `OFTEN_PCT` of its frames. */
export const hitchesOften = (app: AppFrames): boolean => app.hitches * 100 > OFTEN_PCT * app.frames;

export const minutesOf = (c: { start_ms: number; end_ms: number }) =>
  Math.max(1, Math.round((c.end_ms - c.start_ms) / 60_000));

export const framesFailed = (dto: CaptureDto) => dto.frames !== "Ok";
export const kernelFailed = (dto: CaptureDto) => dto.kernel !== "Ok";
/** A source that measured nothing at all, as opposed to one that stopped partway. */
export const framesGone = (dto: CaptureDto) => dto.frames !== "Ok" && "Unavailable" in dto.frames;
export const kernelGone = (dto: CaptureDto) => dto.kernel !== "Ok" && "Unavailable" in dto.kernel;

export type Part = "kernel" | "frames" | "probe";

/** How much of the capture one source measured. */
export interface Coverage {
  part: Part;
  /** Seconds measured, out of `total`. */
  covered: number;
  total: number;
  state: "full" | "partial" | "none";
  /** The source's own words for why it fell short; empty when it measured everything. */
  reason: string;
}

/** Seconds each source measured: the kernel trace and the frames can fall short, the probe runs throughout. */
export function coverageOf(dto: CaptureDto): Coverage[] {
  const total = Math.max(1, Math.round((dto.end_ms - dto.start_ms) / 1000));
  const of = (part: "kernel" | "frames", source: CaptureDto["frames"], empty: boolean): Coverage => {
    if (source === "Ok") {
      return { part, covered: empty ? 0 : total, total, state: empty ? "none" : "full", reason: "" };
    }
    if ("Unavailable" in source) return { part, covered: 0, total, state: "none", reason: source.Unavailable };
    const covered = Math.min(total, source.Partial.covered_secs);
    return { part, covered, total, state: "partial", reason: source.Partial.reason };
  };
  return [
    of("kernel", dto.kernel, dto.drivers.length === 0),
    of("frames", dto.frames, false),
    { part: "probe", covered: total, total, state: "full", reason: "" },
  ];
}

/** Sources that measured less than the whole capture. */
export const shortfalls = (dto: CaptureDto): Coverage[] => coverageOf(dto).filter((c) => c.state !== "full");

/**
 * What the capture can say: it names a culprit, it could not measure everything and so cannot say,
 * it measured everything and saw stalls or an app with many late frames but no culprit, or it saw
 * nothing at all.
 */
export type Verdict = "found" | "incomplete" | "nocul" | "clean";

export function verdictOf(dto: CaptureDto): Verdict {
  if (conclusionOf(dto, driverRows(dto)).kind !== "none") return "found";
  if (shortfalls(dto).length > 0) return "incomplete";
  if (dto.hiccups.length > 0 || dto.apps.some(hitchesOften)) return "nocul";
  return "clean";
}

/** Why a source could not be measured, as the helper worded it. */
export const reasonOf = (source: CaptureDto["frames"]): string => {
  if (source === "Ok") return "";
  return "Unavailable" in source ? source.Unavailable : source.Partial.reason;
};

/** The number that changes between captures: one driver's long calls or one app's late frames. */
export interface Digest {
  kind: "driver" | "app" | "none";
  /** The driver module or the app's exe name. */
  subject: string;
  /** Long calls of the driver, or late frames of the app. */
  count: number;
  /** Longest call in µs, or worst frame in ms. */
  peak: number;
  minutes: number;
}

export function digestOf(dto: CaptureDto): Digest {
  const rows = driverRows(dto);
  const top = conclusionOf(dto, rows);
  const minutes = minutesOf(dto);
  if (top.kind === "driver" || top.kind === "intermediary") {
    return { kind: "driver", subject: top.row.module, count: top.row.over, peak: top.row.maxUs, minutes };
  }
  if (top.kind === "app" && top.app) {
    return { kind: "app", subject: top.app.name, count: top.app.hitches, peak: top.app.worst_ms, minutes };
  }
  return { kind: "none", subject: "", count: 0, peak: 0, minutes };
}

/** The same driver's or app's number in another capture, or `null` when that capture cannot say. */
export function countIn(dto: CaptureDto, digest: Digest): number | null {
  if (digest.kind === "driver") {
    if (kernelFailed(dto)) return null;
    return driverRows(dto).find((r) => r.module.toLowerCase() === digest.subject.toLowerCase())?.over ?? 0;
  }
  if (digest.kind === "app") {
    if (framesFailed(dto)) return null;
    return dto.apps.find((a) => a.name === digest.subject)?.hitches ?? 0;
  }
  return null;
}

export const byNewest = (a: CaptureSummary, b: CaptureSummary) => b.start_ms - a.start_ms;

/** Stalls and late frames that fell within a moment of a call of one driver that was felt on its own. */
export function nearCallsOf(dto: CaptureDto, module: string): { stalls: number; hitches: number } {
  const calls = dto.longest.filter((l) => l.module.toLowerCase() === module.toLowerCase() && l.us >= FELT_US);
  const near = (ts: number) => calls.some((l) => Math.abs(l.ts_ms - ts) <= NEAR_MS);
  return {
    stalls: dto.hiccups.filter((h) => near(h.ts_ms)).length,
    hitches: dto.hitches.filter((h) => near(h.ts_ms)).length,
  };
}
