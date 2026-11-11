import type { IconName } from "../../components/Icon.svelte";
import type { Entry } from "../../lib/capture.svelte";
import { formatDuration, formatNumber, formatTime } from "../../lib/format";
import { t } from "../../lib/i18n.svelte";
import type { AppFrames, CaptureDto, CaptureNote, DetailLevel, Suspect } from "../../lib/types";
import { appName, formatDay, formatDayShort, formatList, formatMs, sameDay } from "../occurrence/format";
import { evidenceLines, isHedged, type Line, lineFor, stepsOf, type Step, type Texts } from "../occurrence/report";
import {
  alignment,
  conclusionOf,
  countIn,
  DPC_LIMIT_US,
  type Digest,
  digestOf,
  type DriverRow,
  driverRows,
  framesFailed,
  hitchesWithCall,
  ISR_LIMIT_US,
  kernelFailed,
  lateApps,
  minutesOf,
  nearCallsOf,
  reasonOf,
} from "./model";

// The words of a deep capture, at the reader's detail level. "PresentMon" and "probe" stay out of Simple.

const k = (key: string, args?: Record<string, string | number>) => t(`ui.captures.${key}`, args);

/** `key.one` for exactly one, `key.other` otherwise: zero is plural in both languages. */
export function count(key: string, n: number, args?: Record<string, string | number>): string {
  return k(`${key}.${n === 1 ? "one" : "other"}`, { count: n, ...args });
}

/** A driver as people know it ("NVIDIA graphics driver"), or its file name when the catalog has none. */
export function driverName(module: string): string {
  const key = `ui.captures.driver.${module.toLowerCase()}`;
  const known = t(key);
  return known === key ? module : known;
}

export const usText = (us: number): string =>
  us >= 999_995 ? formatMs(us / 1000) : us >= 1000 ? `${formatNumber(us / 1000, 2)} ms` : `${formatNumber(us)} µs`;

/** Total time in a driver: milliseconds below a second, seconds above. */
const totalText = (us: number): string =>
  us >= 1_000_000 ? formatDuration(us / 1000) : `${formatNumber(us / 1000)} ms`;

export const frameText = (ms: number): string => formatMs(ms, ms < 10 ? 1 : 0);

/** "50–120 ms" while both ends are under a second, "800 ms–1,2 s" otherwise. */
const rangeText = (min: number, max: number): string =>
  max < 1000 ? `${formatNumber(min)}–${formatNumber(max)} ms` : `${formatMs(min)}–${formatMs(max)}`;

const timesText = (times: number): string => formatNumber(times, times < 10 ? 1 : 0);

const kindText = (kind: "Dpc" | "Isr") => (kind === "Dpc" ? "DPC" : "ISR");
const limitUs = (kind: "Dpc" | "Isr") => (kind === "Dpc" ? DPC_LIMIT_US : ISR_LIMIT_US);

/** The statement forms picked by a detail level: plain words, the numbers, the counters. */
function pick(level: DetailLevel, texts: Texts): string {
  const line = lineFor(level, texts);
  return level === "Technical" ? line.main : level === "Detailed" ? (line.sub ?? line.main) : line.main;
}

/** One statement as a main line and, above Simple, a second one. */
const lineOf = (level: DetailLevel, texts: Texts): Line => lineFor(level, texts);

export type Tone = "busy" | "press" | "freeze";

export interface Note {
  icon: IconName;
  head: string;
  body: string;
  /** The source's own words for why it failed; shown at the Technical level only. */
  reason: string | null;
}

export interface DriverLine {
  key: string;
  name: string;
  module: string;
  suspect: boolean;
  intermediary: boolean;
  kernel: boolean;
  sub: string;
  longest: string;
  over: string;
  /** The longest call against the longest of the table, 0 to 100. */
  bar: number;
  /** Where the recommended limit falls on the same scale, 0 to 100. */
  limit: number;
  /** How many times the recommended limit the longest call took; null when it stayed inside. */
  ratio: string | null;
  dpcCalls: string;
  isrCalls: string;
  total: string;
  longestKind: string;
}

export interface DriverTable {
  rows: DriverLine[];
  /** Names the layers that serve other drivers when the table shows one. */
  layers: string | null;
  /** Modules left out of the table, with their longest call. */
  more: string | null;
}

export interface ResultView {
  crumb: string;
  title: string;
  meta: string;
  minutes: number;
  none: boolean;
  main: {
    icon: IconName;
    name: string;
    sub: string;
    summary: string;
    evidence: Line[];
    layer: boolean;
  } | null;
  notes: Note[];
  steps: Step[];
  drivers: DriverTable | null;
  hasMarks: boolean;
  alignSummary: string;
  apps: { rows: AppLine[]; quiet: string | null } | null;
  /** What the capture measured when nothing stood out. */
  measured: string;
}

export interface AppLine {
  key: string;
  name: string;
  exe: string;
  summary: string;
  detail: string | null;
  app: AppFrames;
}

function confidenceOf(s: Suspect): string {
  return t(`ui.occurrence.confidence.${s.confidence}`);
}

/** The title of the capture's own page. */
function titleOf(dto: CaptureDto, level: DetailLevel, rows: DriverRow[]): string {
  const top = conclusionOf(dto, rows);
  if (top.kind === "driver" || top.kind === "intermediary") {
    const { row } = top;
    const name = driverName(row.module);
    if (level === "Technical") {
      return k("title.driver.tech", { module: row.module, kind: kindText(row.maxKind), us: formatNumber(row.maxUs), over: formatNumber(row.over) });
    }
    if (top.kind === "intermediary") return k("title.layer", { name });
    if (level === "Detailed") return count("title.driver.d", row.over, { name, max: usText(row.maxUs) });
    return k(isHedged("Driver", top.finding.confidence) ? "title.driver.maybe" : "title.driver", { name });
  }
  if (top.kind === "app") {
    const name = appName(top.app?.name ?? "");
    const app = isHedged("Frames", top.finding.confidence) ? "title.app.maybe" : "title.app";
    if (!top.app) return k(app, { name });
    if (level === "Technical") {
      return k("title.app.tech", { exe: top.app.name, hitches: formatNumber(top.app.hitches), frames: formatNumber(top.app.frames), worst: frameText(top.app.worst_ms) });
    }
    if (level === "Detailed") return count("title.app.d", top.app.hitches, { name, worst: frameText(top.app.worst_ms) });
    return k(app, { name });
  }
  return k("title.none");
}

function whenOf(dto: CaptureDto): string {
  return `${formatDay(dto.start_ms)}, ${formatTime(dto.start_ms)} – ${formatTime(dto.end_ms)}`;
}

function evidenceOf(dto: CaptureDto, level: DetailLevel, top: ReturnType<typeof conclusionOf>): Line[] {
  if (top.kind === "app") return evidenceLines(top.finding, level);
  if (top.kind === "none") return [];
  const { row } = top;
  const minutes = minutesOf(dto);
  const kind = kindText(row.maxKind);
  const lines: Line[] = [
    lineOf(level, {
      plain: k("ev.longest.plain", { max: usText(row.maxUs), times: timesText(row.times) }),
      figures: k("ev.longest.figures", { kind, us: formatNumber(row.maxUs), limit: formatNumber(limitUs(row.maxKind)) }),
      tech: k("ev.longest.tech", { kind, us: formatNumber(row.maxUs), limit: formatNumber(limitUs(row.maxKind)) }),
    }),
    lineOf(level, {
      plain: count("ev.over.plain", row.over),
      figures: count("ev.over.figures", row.over, { minutes, rate: formatNumber(row.over / minutes, row.over / minutes < 10 ? 1 : 0) }),
      tech: k("ev.over.tech", { over: formatNumber(row.over), dpc: formatNumber(row.dpc.count), isr: formatNumber(row.isr.count) }),
    }),
  ];
  const near = nearCallsOf(dto, row.module);
  if (dto.hiccups.length && near.stalls > 0) {
    const args = { near: near.stalls };
    lines.push(
      lineOf(level, {
        plain: count("ev.stalls.plain", dto.hiccups.length, args),
        figures: count("ev.stalls.figures", dto.hiccups.length, args),
        tech: k("ev.stalls.tech", { ...args, count: dto.hiccups.length }),
      }),
    );
  }
  if (near.hitches > 0) {
    lines.push(lineOf(level, { plain: count("ev.frames.plain", near.hitches), figures: null, tech: null }));
  }
  if (dto.lost_events > 0) {
    const lost = formatNumber(dto.lost_events);
    lines.push(
      lineOf(level, {
        plain: t("ui.occurrence.ev.loss.plain"),
        figures: t("ui.occurrence.ev.loss.figures", { lost }),
        tech: t("ui.occurrence.ev.loss.tech", { lost }),
      }),
    );
  }
  return lines;
}

function summaryOf(dto: CaptureDto, level: DetailLevel, top: ReturnType<typeof conclusionOf>): string {
  if (top.kind === "none") return "";
  if (top.kind === "app") {
    const name = top.app ? appName(top.app.name) : "";
    const app = top.app;
    return pick(level, {
      plain: t(`ui.occurrence.summary.Frames${isHedged("Frames", top.finding.confidence) ? ".maybe" : ""}`, { app: name }),
      figures: app
        ? k("summary.app.figures", { hitches: formatNumber(app.hitches), frames: formatNumber(app.frames), worst: frameText(app.worst_ms), median: frameText(app.median_ms) })
        : null,
      tech: app
        ? k("summary.app.tech", { exe: app.name, hitches: formatNumber(app.hitches), frames: formatNumber(app.frames), median: frameText(app.median_ms), p99: frameText(app.p99_ms), worst: frameText(app.worst_ms), confidence: confidenceOf(top.finding) })
        : null,
    });
  }
  const { row } = top;
  const near = nearCallsOf(dto, row.module);
  const minutes = minutesOf(dto);
  const kind = kindText(row.maxKind);
  const aligned = near.stalls > 0 || near.hitches > 0;
  if (top.kind === "intermediary") {
    return pick(level, {
      plain: t("ui.occurrence.summary.DriverModule.intermediary", { module: row.module }),
      figures: k("summary.layer.figures", { module: row.module, max: usText(row.maxUs), over: formatNumber(row.over) }),
      tech: k("summary.layer.tech", { module: row.module, kind, us: formatNumber(row.maxUs), over: formatNumber(row.over), calls: formatNumber(row.calls), confidence: confidenceOf(top.finding) }),
    });
  }
  return pick(level, {
    plain: k(aligned ? "summary.driver.aligned" : "summary.driver"),
    figures: k("summary.driver.figures", { over: formatNumber(row.over), max: usText(row.maxUs), times: timesText(row.times) }),
    tech: k("summary.driver.tech", {
      module: row.module,
      dpc: formatNumber(row.dpc.count),
      isr: formatNumber(row.isr.count),
      kind,
      us: formatNumber(row.maxUs),
      limit: formatNumber(limitUs(row.maxKind)),
      over: formatNumber(row.over),
      rate: formatNumber(row.over / minutes, row.over / minutes < 10 ? 1 : 0),
      confidence: confidenceOf(top.finding),
    }),
  });
}

function notesOf(dto: CaptureDto, level: DetailLevel): Note[] {
  const notes: Note[] = [];
  if (dto.lost_events > 0) {
    const lost = formatNumber(dto.lost_events);
    notes.push({
      icon: "info",
      head: pick(level, { plain: k("note.lost.head"), figures: k("note.lost.head.d", { lost }), tech: k("note.lost.head.t", { lost }) }),
      body: pick(level, { plain: k("note.lost.body"), figures: k("note.lost.body.d"), tech: k("note.lost.body.t") }),
      reason: null,
    });
  }
  if (framesFailed(dto)) {
    notes.push({
      icon: "app",
      head: level === "Simple" ? k("note.frames.head") : k("note.frames.head.d"),
      body: level === "Simple" ? k("note.frames.body") : k("note.frames.body.d"),
      reason: level === "Technical" ? reasonOf(dto.frames) : null,
    });
  }
  if (kernelFailed(dto)) {
    notes.push({
      icon: "chip",
      head: k("note.kernel.head"),
      body: level === "Simple" ? k("note.kernel.body") : k("note.kernel.body.d"),
      reason: level === "Technical" ? reasonOf(dto.kernel) : null,
    });
  }
  return notes;
}

/** What to do after this capture, each step once. */
export function stepsFor(dto: CaptureDto, level: DetailLevel): Step[] {
  const steps = stepsOf(dto.findings);
  const top = conclusionOf(dto, driverRows(dto));
  if (steps.length) steps.push({ key: "again", text: k("step.again") });
  if (top.kind === "intermediary") steps.push({ key: "layer", text: k("step.layer", { module: top.row.module }) });
  const app = dto.findings.find((s) => typeof s.cause !== "string" && "Frames" in s.cause);
  if (app && typeof app.cause !== "string" && "Frames" in app.cause) {
    steps.push({ key: `app:${app.cause.Frames.app}`, text: k("step.app", { app: appName(app.cause.Frames.app) }) });
  }
  if (dto.lost_events > 0) steps.push({ key: "lost", text: k("step.lost") });
  if (level === "Technical" && steps.some((s) => s.key.startsWith("UpdateDriver"))) {
    steps.push({ key: "overclock", text: k("step.overclock") });
  }
  if (top.kind === "none") steps.push({ key: "again.none", text: k("step.none") });
  return steps;
}

function driverTable(dto: CaptureDto, level: DetailLevel, all: DriverRow[]): DriverTable {
  const limit = level === "Simple" ? 3 : level === "Detailed" ? 5 : 15;
  const nameable = all.filter((r) => !r.kernel && r.module !== "<unknown>");
  const shown = (level === "Technical" ? all : nameable).slice(0, limit);
  // A full bar is the worst longest call, or the recommended limit when every driver stayed inside it.
  const widest = Math.max(1, ...shown.map((r) => r.times));
  const rows = shown.map((r): DriverLine => {
    const name = driverName(r.module);
    const max = usText(r.maxUs);
    const sub = r.kernel
      ? k("driver.sub.kernel")
      : r.suspect
        ? pick(level, {
            plain: count("driver.sub.suspect", r.over, { module: r.module, max, times: timesText(r.times) }),
            figures: k("driver.sub.suspect.d", { module: r.module, times: timesText(r.times) }),
            tech: name === r.module ? null : name,
          })
        : r.intermediary
          ? pick(level, {
              plain: k("driver.sub.layer", { module: r.module }),
              figures: k("driver.sub.layer.d", { module: r.module }),
              tech: name === r.module ? null : name,
            })
          : pick(level, {
              plain: k(r.over === 0 ? "driver.sub.fine" : "driver.sub.some", { module: r.module }),
              figures: k(r.over === 0 ? "driver.sub.fine.d" : "driver.sub.some.d", { module: r.module }),
              tech: name === r.module ? null : name,
            });
    return {
      key: r.module,
      name: level === "Technical" ? r.module : name,
      module: r.module,
      suspect: r.suspect,
      intermediary: r.intermediary,
      kernel: r.kernel,
      sub,
      longest: max,
      over: formatNumber(r.over),
      bar: Math.max(3, (r.times / widest) * 100),
      limit: (1 / widest) * 100,
      ratio: r.times > 1 ? timesText(r.times) : null,
      dpcCalls: r.dpc.count ? formatNumber(r.dpc.count) : "—",
      isrCalls: r.isr.count ? formatNumber(r.isr.count) : "—",
      total: totalText(r.totalUs),
      longestKind: kindText(r.maxKind),
    };
  });
  const present = shown.filter((r) => r.intermediary).map((r) => r.module);
  const left = (level === "Technical" ? all : nameable).slice(limit);
  const widestLeft = Math.max(0, ...left.map((r) => r.maxUs));
  return {
    rows,
    layers: present.length ? count("driver.layers", present.length, { modules: formatList(present) }) : null,
    more: level === "Technical" && left.length ? count("driver.more", left.length, { max: usText(widestLeft) }) : null,
  };
}

function appLines(dto: CaptureDto, level: DetailLevel): { rows: AppLine[]; quiet: string | null } {
  const late = lateApps(dto);
  const rows = late.map((app): AppLine => {
    const name = appName(app.name);
    const together = hitchesWithCall(dto, app);
    return {
      key: `${app.name}:${app.pid}`,
      name,
      exe: app.name,
      app,
      summary: pick(level, {
        plain: count("app.summary", app.hitches, { frames: formatNumber(app.frames), worst: frameText(app.worst_ms), median: frameText(app.median_ms) }),
        figures: k("app.summary.d", { frames: formatNumber(app.frames), median: frameText(app.median_ms), hitches: formatNumber(app.hitches), worst: frameText(app.worst_ms) }),
        tech: null,
      }),
      detail:
        together === null
          ? null
          : together === 0
            ? k("app.alone")
            : pick(level, {
                plain:
                  together === app.hitches
                    ? k("app.together.all")
                    : k("app.together", { together, rest: app.hitches - together }),
                figures: k("app.together.d", { together, hitches: app.hitches }),
                tech: null,
              }),
    };
  });
  const quietApps = dto.apps.filter((a) => a.hitches === 0);
  const quiet = level !== "Simple" && quietApps.length
    ? k("app.quiet", { apps: formatList(quietApps.map((a) => appName(a.name))), frames: formatNumber(quietApps.reduce((sum, a) => sum + a.frames, 0)) })
    : null;
  return { rows, quiet };
}

function alignSummary(dto: CaptureDto, level: DetailLevel, rows: DriverRow[]): string {
  const al = alignment(dto);
  if (al.stalls === 0) {
    return pick(level, { plain: k("align.calm"), figures: k("align.calm.d"), tech: k("align.calm.t") });
  }
  const times = formatList(dto.hiccups.map((h) => formatTime(h.ts_ms)));
  const range = rangeText(al.minStallMs, al.maxStallMs);
  const parts = [
    pick(level, {
      plain: count("align.stalls", al.stalls),
      figures: count("align.stalls.d", al.stalls, { range, times }),
      tech: k("align.stalls.t", { count: al.stalls, lags: dto.hiccups.map((h) => formatMs(h.lag_ms, 1)).join(", ") }),
    }),
  ];
  const top = conclusionOf(dto, rows);
  const driver = top.kind === "driver" || top.kind === "intermediary" ? top.row.module : null;
  const withCall = al.stallsWithCall;
  const single = al.stalls === 1;
  if (dto.longest.length && withCall > 0) {
    parts.push(
      pick(level, {
        plain: single ? k("align.calls.single") : count("align.calls", withCall),
        figures: single ? k("align.calls.single.d") : count("align.calls.d", withCall),
        tech: k("align.calls.t", { count: withCall, stalls: al.stalls }),
      }),
    );
  } else if (driver) {
    parts.push(pick(level, { plain: k("align.nocalls"), figures: k("align.nocalls"), tech: k("align.nocalls") }));
  }
  if (!framesFailed(dto) && al.stallsWithFrame > 0) {
    parts.push(
      pick(level, {
        plain: single ? k("align.frames.single") : count("align.frames", al.stallsWithFrame),
        figures: single ? k("align.frames.single.d") : count("align.frames.d", al.stallsWithFrame),
        tech: k("align.frames.t", { count: al.stallsWithFrame, stalls: al.stalls }),
      }),
    );
  }
  if (framesFailed(dto)) parts.push(k("align.noframes"));
  return parts.join(" ");
}

export function buildResult(dto: CaptureDto, level: DetailLevel): ResultView {
  const all = driverRows(dto);
  const top = conclusionOf(dto, all);
  const al = alignment(dto);
  const minutes = minutesOf(dto);
  const name =
    top.kind === "driver" || top.kind === "intermediary"
      ? driverName(top.row.module)
      : top.kind === "app"
        ? appName(top.app?.name ?? "")
        : "";
  const sub =
    top.kind === "driver" || top.kind === "intermediary"
      ? [name === top.row.module ? null : top.row.module, top.kind === "intermediary" ? k("layer.pill") : null, confidenceOf(top.finding)]
      : top.kind === "app"
        ? [top.app?.name ?? null, confidenceOf(top.finding)]
        : [];
  const measuredDrivers = all.filter((r) => !r.kernel && r.module !== "<unknown>").length;
  const frames = dto.apps.reduce((sum, a) => sum + a.frames, 0);
  return {
    crumb: sameDay(dto.start_ms, Date.now()) ? formatTime(dto.start_ms) : `${formatDayShort(dto.start_ms)}, ${formatTime(dto.start_ms)}`,
    title: titleOf(dto, level, all),
    meta: k("meta", { when: whenOf(dto), minutes }),
    minutes,
    none: top.kind === "none",
    main:
      top.kind === "none"
        ? null
        : {
            icon: top.kind === "app" ? "app" : "chip",
            name,
            sub: sub.filter(Boolean).join(" · "),
            summary: summaryOf(dto, level, top),
            evidence: evidenceOf(dto, level, top),
            layer: top.kind === "intermediary",
          },
    notes: notesOf(dto, level),
    steps: stepsFor(dto, level),
    drivers: kernelFailed(dto) ? null : driverTable(dto, level, all),
    hasMarks: al.marks.length > 0 || dto.hiccups.length > 0,
    alignSummary: alignSummary(dto, level, all),
    apps: framesFailed(dto) ? null : appLines(dto, level),
    measured: k(frames > 0 ? "measured.frames" : "measured", { drivers: formatNumber(measuredDrivers), frames: formatNumber(frames), minutes }),
  };
}

/** The line a mark gets in the alignment chart's readout, at a detail level. */
export function markLine(kind: 0 | 1 | 2, value: number, who: string, level: DetailLevel, median: number | null): string {
  if (kind === 0) {
    return pick(level, {
      plain: k("mark.stall.plain", { ms: formatMs(value) }),
      figures: k("mark.stall.d", { ms: formatMs(value) }),
      tech: k("mark.stall.t", { ms: formatMs(value, 1) }),
    });
  }
  if (kind === 1) {
    return pick(level, {
      plain: k("mark.call.plain", { name: driverName(who), max: usText(value) }),
      figures: k("mark.call.d", { module: who, us: usText(value) }),
      tech: k("mark.call.t", { module: who, us: formatNumber(value) }),
    });
  }
  const app = appName(who);
  const normal = median === null ? "" : frameText(median);
  return pick(level, {
    plain: median === null ? k("mark.frame.plain.short", { app, ms: frameText(value) }) : k("mark.frame.plain", { app, ms: frameText(value), median: normal }),
    figures: median === null ? k("mark.frame.d.short", { app, ms: frameText(value) }) : k("mark.frame.d", { app, ms: frameText(value), median: normal }),
    tech: k("mark.frame.t", { exe: who, ms: formatMs(value, 1) }),
  });
}

export interface RowView {
  id: number;
  icon: IconName;
  /** Nothing stood out in this capture. */
  quiet: boolean;
  title: string;
  sub: string;
  day: string;
  time: string;
}

/** One history row: what the capture named and the number that changes between captures. */
export function rowOf(entry: Entry): RowView {
  const { dto, summary } = entry;
  const d = digestOf(dto);
  const minutes = minutesOf(dto);
  const parts: string[] = [];
  let title = k("row.none.title");
  let icon: IconName = "wave";
  if (d.kind === "driver") {
    title = driverName(d.subject);
    icon = "chip";
    parts.push(count("row.driver", d.count, { max: usText(d.peak) }));
  } else if (d.kind === "app") {
    title = appName(d.subject);
    icon = "app";
    parts.push(count("row.app", d.count, { worst: frameText(d.peak) }));
  } else {
    parts.push(k("row.none"));
  }
  parts.push(k("row.minutes", { minutes }));
  if (dto.lost_events > 0) parts.push(k("row.lost"));
  if (framesFailed(dto)) parts.push(k("row.noframes"));
  if (kernelFailed(dto)) parts.push(k("row.nokernel"));
  return {
    id: summary.id,
    icon,
    quiet: d.kind === "none",
    title,
    sub: parts.join(" · "),
    day: sameDay(summary.start_ms, Date.now()) ? k("today") : formatDayShort(summary.start_ms),
    time: formatTime(summary.start_ms),
  };
}

/** The latest capture against the one before: the same driver's long calls or app's late frames. */
export function compareLine(entries: Entry[]): string | null {
  if (entries.length < 2) return null;
  const [latest, before] = entries;
  const dl = digestOf(latest.dto);
  const db = digestOf(before.dto);
  const base: Digest = db.kind !== "none" ? db : dl;
  if (base.kind === "none") return null;
  const fromBefore = base === db;
  const from = fromBefore ? db.count : countIn(before.dto, dl);
  const to = fromBefore ? countIn(latest.dto, db) : dl.count;
  if (from === null || to === null) return null;
  const name = base.kind === "driver" ? driverName(base.subject) : appName(base.subject);
  const noun = base.kind === "driver" ? "calls" : "frames";
  const minutesBefore = digestOf(before.dto).minutes;
  const minutesLatest = digestOf(latest.dto).minutes;
  if (minutesBefore === minutesLatest) {
    return count(`compare.${noun}`, to, { name, from: formatNumber(from), to: formatNumber(to) });
  }
  const rate = (n: number, minutes: number) => n / minutes;
  const show = (n: number) => formatNumber(n, n < 10 ? 1 : 0);
  const rateTo = rate(to, minutesLatest);
  return count(`compare.${noun}.rate`, rateTo === 1 ? 1 : 2, {
    name,
    from: show(rate(from, minutesBefore)),
    to: show(rateTo),
  });
}

/** A readable header for the copied result: Markdown with the plain words and the counters. */
export function resultMarkdown(dto: CaptureDto): string {
  const simple = buildResult(dto, "Simple");
  const tech = buildResult(dto, "Technical");
  const lines: string[] = [`## ${simple.title}`, "", `${simple.meta}`, ""];
  if (simple.main) lines.push(simple.main.summary, "");
  for (const note of tech.notes) lines.push(`> ${note.head}. ${note.body}${note.reason ? ` (${note.reason})` : ""}`, "");
  if (tech.main) {
    lines.push(`### ${k("md.evidence")}`, "");
    for (const e of tech.main.evidence) lines.push(`- ${e.main}${e.sub ? ` — ${e.sub}` : ""}`);
    lines.push("");
  }
  if (tech.drivers && tech.drivers.rows.length) {
    lines.push(`### ${k("md.drivers")}`, "", `| ${k("md.col.module")} | DPC | ISR | ${k("md.col.longest")} | ${k("md.col.over")} |`, "|---|---:|---:|---:|---:|");
    for (const r of tech.drivers.rows) lines.push(`| ${r.module} | ${r.dpcCalls} | ${r.isrCalls} | ${r.longest} ${r.longestKind} | ${r.over} |`);
    lines.push("");
  }
  if (tech.apps && tech.apps.rows.length) {
    lines.push(`### ${k("md.apps")}`, "");
    for (const a of tech.apps.rows) {
      lines.push(`- ${a.exe}: ${formatNumber(a.app.hitches)} / ${formatNumber(a.app.frames)}, ${frameText(a.app.worst_ms)}`);
    }
    lines.push("");
  }
  lines.push(`### ${k("md.time")}`, "", tech.alignSummary, "", `### ${k("md.steps")}`, "");
  for (const step of simple.steps) lines.push(`- ${step.text}`);
  lines.push("", `_${k("md.footer")}_`);
  return lines.join("\n");
}

/** The line a report carries when a capture covered its window: who the capture names for those minutes. */
export function coveredLine(note: CaptureNote): string {
  const names = note.suspects.flatMap((s) => {
    if (typeof s.cause === "string") return [];
    if ("DriverModule" in s.cause) return [driverName(s.cause.DriverModule.module)];
    if ("Frames" in s.cause) return [appName(s.cause.Frames.app)];
    return [];
  });
  const time = formatTime(note.start_ms);
  return names.length
    ? k("covered.names", { time, names: formatList(names.slice(0, 2)) })
    : k("covered.none", { time });
}
