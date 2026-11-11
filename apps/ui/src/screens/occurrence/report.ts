import type { IconName } from "../../components/Icon.svelte";
import { formatClock, formatDuration, formatNumber, formatPercent, formatTime } from "../../lib/format";
import { t, tn } from "../../lib/i18n.svelte";
import type {
  Cause,
  Confidence,
  CrashKind,
  DetailLevel,
  Evidence,
  FactKind,
  Metric,
  Occurrence,
  OccurrenceDto,
  Suspect,
  Trigger,
} from "../../lib/types";
import {
  appName,
  diskName,
  formatBytes,
  formatDay,
  formatDayShort,
  formatList,
  formatMs,
  formatRate,
  formatStamp,
  sameDay,
} from "./format";
import { coveredLine } from "../capture/words";
import { causeName as shortCause } from "../today/text";

// The words of a report, built from its typed facts and evidence at the reader's detail level.

export type Tone = "busy" | "press" | "freeze";
export type CauseKind =
  | "Storage"
  | "DiskIo"
  | "Memory"
  | "Cpu"
  | "Driver"
  | "GpuDriver"
  | "Throttling"
  | "App"
  | "Unclear"
  | "DriverModule"
  | "Frames"
  | "BlueScreen"
  | "Unresponsive"
  | "SleepTransition"
  | "PowerCut"
  | "HardwareFault"
  | "NotMeasured";

/** One statement in the forms the detail levels pick from: plain words, the numbers, the counters. */
export interface Texts {
  plain: string;
  figures: string | null;
  tech: string | null;
}

/** What one detail level shows of a statement: a main line and, above Simple, a second one. */
export interface Line {
  main: string;
  sub: string | null;
}

export interface SuspectView {
  icon: IconName;
  name: string;
  sub: string;
  confidence: string;
  evidence: Line[];
}

export interface Step {
  key: string;
  text: string;
}

export interface FactView {
  time: string;
  datetime: string;
  line: Line;
  /** The PC or an app stopped answering here. */
  mark: boolean;
}

export interface ReportView {
  crumb: string;
  title: string;
  meta: string;
  /** The date and time alone, and the "sensitive mode" note when it applies. */
  when: string;
  /** The kind with its gravity: "Severe freeze". */
  severity: string;
  /** What it happened in, in a few words: "C: disk". */
  where: string;
  figure: string;
  figureCaption: string;
  tone: Tone;
  cause: CauseKind;
  main: SuspectView;
  others: SuspectView[];
  summary: string;
  note: string | null;
  steps: Step[];
  facts: FactView[];
}

type Args = Record<string, string | number>;

const k = (key: string, args?: Args) => t(`ui.occurrence.${key}`, args);
const kn = (key: string, count: number, args?: Args) => tn(`ui.occurrence.${key}`, count, args);

/** Below these an app that stopped answering was idle, not busy. */
const IDLE_CPU_PCT = 5;
const IDLE_IO_BPS = 1024 * 1024;

const ICON: Record<CauseKind, IconName> = {
  Storage: "disk",
  DiskIo: "disk",
  Memory: "memory",
  Cpu: "chip",
  Driver: "chip",
  GpuDriver: "gpu",
  Throttling: "chip",
  App: "app",
  Unclear: "chip",
  DriverModule: "chip",
  Frames: "app",
  BlueScreen: "power",
  Unresponsive: "power",
  SleepTransition: "power",
  PowerCut: "power",
  HardwareFault: "chip",
  NotMeasured: "chip",
};

/** Metrics that cross by falling below their threshold. */
const FALLING: readonly Metric[] = ["MemAvailPct", "PerfLimitPct"];
const METRIC_EVIDENCE: readonly Metric[] = [
  "DiskLatencyMs",
  "DiskQueue",
  "DiskActivePct",
  "MemAvailPct",
  "HardFaultsPerS",
  "CpuPct",
  "CoreDpcIsrPct",
  "GpuPct",
  "PerfLimitPct",
];
const PROCESS_METRICS: readonly Metric[] = ["CpuPct", "IoBytesPerS", "PrivateBytes", "GpuPct"];
const CROSSED_METRICS: readonly Metric[] = [
  "CpuPct",
  "MemAvailPct",
  "HardFaultsPerS",
  "DiskLatencyMs",
  "DiskQueue",
  "CoreDpcIsrPct",
  "GpuPct",
  "PerfLimitPct",
];
const BUSIEST_METRICS: readonly Metric[] = ["CpuPct", "PrivateBytes", "IoBytesPerS"];

export function lineFor(level: DetailLevel, texts: Texts): Line {
  if (level === "Simple") return { main: texts.plain, sub: null };
  if (level === "Detailed") return { main: texts.plain, sub: texts.figures };
  const counters = texts.tech ?? texts.figures;
  return counters ? { main: counters, sub: texts.plain } : { main: texts.plain, sub: null };
}

export function causeKind(cause: Cause): CauseKind {
  return typeof cause === "string" ? cause : (Object.keys(cause)[0] as CauseKind);
}

/** The state colour of an occurrence: a hung app and a severe freeze are the strongest. */
export function toneOf(o: Occurrence): Tone {
  if (o.kind === "AppHang" || o.severity === "Severe") return "freeze";
  return o.severity === "Medium" ? "press" : "busy";
}

export function metricValue(metric: Metric, value: number): string {
  switch (metric) {
    case "DiskLatencyMs":
      return formatMs(value);
    case "DiskQueue":
    case "HardFaultsPerS":
      return formatNumber(value);
    case "PrivateBytes":
      return formatBytes(value);
    case "IoBytesPerS":
      return formatRate(value);
    default:
      return formatPercent(value, value > 0 && value < 10 ? 1 : 0);
  }
}

function past(metric: Metric, value: number, threshold: number): boolean {
  return FALLING.includes(metric) ? value < threshold : value >= threshold;
}

/** The disk or core a metric was read on, as the reader knows it. */
function subjectOf(metric: Metric, subject: string | null): string {
  if (!subject) return "";
  return metric === "CoreDpcIsrPct" ? k("core", { n: subject }) : diskName(subject);
}

/** Metrics read on one disk or core, named with it. */
const PER_SUBJECT: readonly Metric[] = ["DiskLatencyMs", "DiskQueue", "DiskActivePct", "CoreDpcIsrPct"];

function metricName(metric: Metric, subject: string | null): string {
  const where = subjectOf(metric, subject);
  return where && PER_SUBJECT.includes(metric) ? k(`metric.${metric}.on`, { subject: where }) : k(`metric.${metric}`);
}

function evidenceTexts(evidence: Evidence, cause: CauseKind): Texts {
  if ("Event" in evidence) {
    const { ts_ms, provider, id, meaning } = evidence.Event;
    const time = formatClock(ts_ms);
    return {
      plain: k(`event.${meaning}`),
      figures: k("ev.event.figures", { time }),
      tech: k("ev.event.tech", { provider, id: String(id), time }),
    };
  }
  if ("Metric" in evidence) {
    const { ts_ms, metric, value, threshold, subject } = evidence.Metric;
    const shown = metricValue(metric, value);
    const tech = k("ev.metric.tech", {
      name: metricName(metric, subject),
      value: shown,
      threshold: metricValue(metric, threshold),
      time: formatClock(ts_ms),
    });
    if (!METRIC_EVIDENCE.includes(metric)) return { plain: tech, figures: null, tech: null };
    const side = past(metric, value, threshold) ? "over" : "under";
    return {
      plain: k(`ev.${metric}.${side}`, { subject: subjectOf(metric, subject) }),
      figures: k(`ev.${metric}.figures`, { value: shown }),
      tech,
    };
  }
  if ("Process" in evidence) {
    const { pid, name, metric, value } = evidence.Process;
    const app = appName(name);
    const shown = metricValue(metric, value);
    const known = PROCESS_METRICS.includes(metric);
    return {
      plain: processPlain(cause, metric, value, app),
      figures: known ? k(`ev.process.${metric}.figures`, { value: shown }) : shown,
      tech: k("ev.process.tech", { exe: name, pid: String(pid), name: k(`metric.${metric}`), value: shown }),
    };
  }
  if ("Routine" in evidence) {
    const r = evidence.Routine;
    const max = formatUs(r.max_us);
    return {
      plain: k("ev.routine.plain", { module: r.module, max }),
      figures: k("ev.routine.figures", { slow: formatNumber(r.at_least_100us), max }),
      tech: k("ev.routine.tech", {
        kind: r.kind.toUpperCase(),
        module: r.module,
        count: formatNumber(r.count),
        max_us: formatNumber(r.max_us),
        slow: formatNumber(r.at_least_100us),
        long: formatNumber(r.at_least_1ms),
        core: formatPercent(r.core_pct),
      }),
    };
  }
  if ("LongRoutine" in evidence) {
    const { ts_ms, module, kind, us } = evidence.LongRoutine;
    const time = formatClock(ts_ms);
    return {
      plain: k("ev.longroutine.plain", { module, time }),
      figures: k("ev.longroutine.figures", { us: formatUs(us), time }),
      tech: k("ev.longroutine.tech", { kind: kind.toUpperCase(), module, us: formatNumber(us), time }),
    };
  }
  if ("Frames" in evidence) {
    const f = evidence.Frames;
    const app = appName(f.app);
    return {
      plain: kn("ev.frames.plain", f.hitches, { app }),
      figures: k("ev.frames.figures", {
        hitches: formatNumber(f.hitches),
        frames: formatNumber(f.frames),
        worst: formatMs(f.worst_ms),
      }),
      tech: k("ev.frames.tech", {
        median: formatMs(f.median_ms),
        worst: formatMs(f.worst_ms),
        hitches: formatNumber(f.hitches),
        frames: formatNumber(f.frames),
      }),
    };
  }
  if ("Hitch" in evidence) {
    const { ts_ms, app, frame_ms } = evidence.Hitch;
    const args = { app: appName(app), ms: formatMs(frame_ms), time: formatClock(ts_ms) };
    return { plain: k("ev.hitch.plain", args), figures: k("ev.hitch.figures", args), tech: k("ev.hitch.tech", args) };
  }
  if ("Hiccup" in evidence) {
    const { ts_ms, lag_ms } = evidence.Hiccup;
    const args = { ms: formatMs(lag_ms), time: formatClock(ts_ms) };
    return { plain: k("ev.hiccup.plain", args), figures: k("ev.hiccup.figures", args), tech: k("ev.hiccup.tech", args) };
  }
  if ("CaptureLoss" in evidence) {
    const lost = formatNumber(evidence.CaptureLoss.lost_events);
    return { plain: k("ev.loss.plain"), figures: k("ev.loss.figures", { lost }), tech: k("ev.loss.tech", { lost }) };
  }
  const { name, duration_ms } = evidence.Hang;
  const duration = formatDuration(duration_ms);
  return {
    plain: k("ev.hang.plain", { app: appName(name) }),
    figures: k("ev.hang.figures", { duration }),
    tech: k("ev.hang.tech", { duration }),
  };
}

function processPlain(cause: CauseKind, metric: Metric, value: number, app: string): string {
  if (cause === "App" && metric === "CpuPct") {
    return k(`ev.app.CpuPct.${value < IDLE_CPU_PCT ? "idle" : "busy"}`, { app });
  }
  if (cause === "App" && metric === "IoBytesPerS") {
    return k(`ev.app.IoBytesPerS.${value < IDLE_IO_BPS ? "idle" : "busy"}`, { app });
  }
  if (cause === "Cpu" && metric === "CpuPct") return k("ev.process.CpuPct.among", { app });
  if (!PROCESS_METRICS.includes(metric)) return k("ev.process.other", { app });
  return k(`ev.process.${metric}.plain`, { app });
}

function mediaOf(o: Occurrence, disk: string): string {
  const media = o.window.machine.disks.find((d) => d.name === disk)?.media;
  return media === "Ssd" ? "SSD" : media === "Hdd" ? "HDD" : "";
}

/** The suspect's name and the parts of its second line. */
function causeName(cause: Cause, o: Occurrence): { name: string; parts: string[] } {
  if (typeof cause === "string") {
    return { name: k(`cause.${cause}.name`), parts: cause === "Throttling" ? [k("cause.Throttling.sub")] : [] };
  }
  if ("Storage" in cause) {
    const disk = cause.Storage.disk;
    return disk
      ? { name: k("cause.Storage.name", { disk: diskName(disk) }), parts: [mediaOf(o, disk)] }
      : { name: k("cause.Storage.name_any"), parts: [] };
  }
  if ("DiskIo" in cause) {
    const { disk, hdd, process } = cause.DiskIo;
    return {
      name: k("cause.DiskIo.name", { disk: diskName(disk) }),
      parts: [hdd ? "HDD" : mediaOf(o, disk), process ? k("cause.DiskIo.who", { app: appName(process) }) : ""],
    };
  }
  if ("Memory" in cause) {
    const process = cause.Memory.process;
    return {
      name: k("cause.Memory.name"),
      parts: [process ? k("cause.Memory.who", { app: appName(process) }) : ""],
    };
  }
  if ("Cpu" in cause) {
    return { name: k("cause.Cpu.name"), parts: [formatList(cause.Cpu.processes.map(appName))] };
  }
  if ("Driver" in cause) {
    return { name: k("cause.Driver.name"), parts: [k("cause.Driver.sub", { core: k("core", { n: String(cause.Driver.core) }) })] };
  }
  if ("DriverModule" in cause) {
    const { module, intermediary } = cause.DriverModule;
    return { name: module, parts: [k(intermediary ? "cause.DriverModule.intermediary" : "cause.DriverModule.sub")] };
  }
  if ("Frames" in cause) {
    return { name: appName(cause.Frames.app), parts: [k("cause.Frames.sub")] };
  }
  if ("BlueScreen" in cause) {
    const code = bugcheck(cause.BlueScreen.code);
    return { name: k("cause.BlueScreen.name"), parts: [k("cause.BlueScreen.sub", { code })] };
  }
  if ("PowerCut" in cause) {
    return { name: k("cause.PowerCut.name"), parts: [cause.PowerCut.no_dump ? k("cause.PowerCut.no_dump") : ""] };
  }
  return { name: appName(cause.App.name), parts: [cause.App.name] };
}

/** A duration in microseconds, as milliseconds once it reaches one. */
function formatUs(us: number): string {
  return us >= 1000 ? formatMs(us / 1000) : k("us", { us: formatNumber(us) });
}

/** The evidence of one suspect at a detail level, for pages that show a suspect outside a report. */
export function evidenceLines(s: Suspect, level: DetailLevel): Line[] {
  const kind = causeKind(s.cause);
  return s.evidence.map((e) => lineFor(level, evidenceTexts(e, kind)));
}

function suspectView(s: Suspect, o: Occurrence, level: DetailLevel): SuspectView {
  const kind = causeKind(s.cause);
  const { name, parts } = causeName(s.cause, o);
  return {
    icon: ICON[kind],
    name,
    sub: parts.filter(Boolean).join(" · "),
    confidence: k(`confidence.${s.confidence}`),
    evidence: s.evidence.map((e) => lineFor(level, evidenceTexts(e, kind))),
  };
}

/** Causes that name a mechanism: below High they are worded as a possibility, not a fact. */
const MECHANISM: readonly CauseKind[] = ["Driver", "DriverModule", "Frames", "Throttling", "Cpu", "Memory"];

/** Whether a sentence for this cause must say it "may" be so at this confidence. */
export function isHedged(kind: CauseKind, confidence: Confidence): boolean {
  return confidence !== "High" && MECHANISM.includes(kind);
}

/** The gravest disk event among the evidence: a bad block or paging error, then a reset, then a retry. */
export function storageMeaning(evidence: Evidence[]): "media" | "reset" | "retry" {
  const meanings = evidence.flatMap((e) => ("Event" in e ? [e.Event.meaning] : []));
  if (meanings.some((m) => m === "DiskBadBlock" || m === "PagingError")) return "media";
  if (meanings.includes("DiskRetry") && !meanings.includes("StorageReset")) return "retry";
  return "reset";
}

function summaryOf(cause: Cause, confidence: Confidence, evidence: Evidence[]): string {
  const maybe = isHedged(causeKind(cause), confidence) ? ".maybe" : "";
  if (typeof cause === "string") return k(`summary.${cause}${maybe}`);
  if ("Storage" in cause) {
    const disk = cause.Storage.disk;
    const meaning = storageMeaning(evidence);
    const hedge = confidence === "High" ? "" : ".medium";
    const key = meaning === "reset" ? `summary.Storage${hedge}` : `summary.Storage.${meaning}${hedge}`;
    // The reset sentence names "the disk C:"; the others follow the word "disk" and take the bare name.
    const named = meaning === "reset" ? (disk ? k("disk_the", { disk: diskName(disk) }) : k("disk_any")) : disk ? diskName(disk) : "";
    return k(key, { disk: named });
  }
  if ("DiskIo" in cause) {
    const { disk, hdd, process } = cause.DiskIo;
    return [
      k("summary.DiskIo", { disk: diskName(disk) }),
      hdd ? k("summary.DiskIo.hdd") : "",
      process ? k("summary.DiskIo.who", { app: appName(process) }) : "",
    ]
      .filter(Boolean)
      .join(" ");
  }
  if ("Memory" in cause) {
    const process = cause.Memory.process;
    return [k(`summary.Memory${maybe}`), process ? k("summary.Memory.who", { app: appName(process) }) : ""]
      .filter(Boolean)
      .join(" ");
  }
  if ("Cpu" in cause) {
    const apps = cause.Cpu.processes.map(appName);
    return [k(`summary.Cpu${maybe}`), apps.length ? kn("summary.Cpu.who", apps.length, { apps: formatList(apps) }) : ""]
      .filter(Boolean)
      .join(" ");
  }
  if ("Driver" in cause) return k(`summary.Driver${maybe}`);
  if ("DriverModule" in cause) {
    const { module, intermediary } = cause.DriverModule;
    return k(intermediary ? "summary.DriverModule.intermediary" : `summary.DriverModule${maybe}`, { module });
  }
  if ("Frames" in cause) return k(`summary.Frames${maybe}`, { app: appName(cause.Frames.app) });
  if ("BlueScreen" in cause) return k("summary.BlueScreen", { code: bugcheck(cause.BlueScreen.code) });
  if ("PowerCut" in cause) {
    return [k("summary.PowerCut"), cause.PowerCut.no_dump ? k("summary.PowerCut.no_dump") : ""]
      .filter(Boolean)
      .join(" ");
  }
  return k("summary.App", { app: appName(cause.App.name) });
}

/** A sentence when naming the cause takes the deep capture, which this version does not have. */
function noteOf(suspects: Suspect[]): string | null {
  if (!suspects.some((s) => s.next_steps.includes("DeepCapture"))) return null;
  return k(suspects.some((s) => causeKind(s.cause) === "Driver") ? "note.Driver" : "note.default");
}

/**
 * Every suspect's next steps, main suspect first, each once; the deep capture is the note instead.
 * A blue screen's own steps give way to `fromDump`, the steps of its dump once that is read.
 */
export function stepsOf(suspects: Suspect[], fromDump: Step[] | null = null): Step[] {
  const steps: Step[] = [];
  for (const s of suspects) {
    if (fromDump && causeKind(s.cause) === "BlueScreen") {
      for (const step of fromDump) {
        const key = `dump:${step.key}`;
        if (!steps.some((known) => known.key === key)) steps.push({ key, text: step.text });
      }
      continue;
    }
    const app = typeof s.cause !== "string" && "App" in s.cause ? s.cause.App.name : null;
    const driver = typeof s.cause !== "string" && "DriverModule" in s.cause ? s.cause.DriverModule.module : null;
    const disk = typeof s.cause !== "string" && "Storage" in s.cause ? s.cause.Storage.disk : null;
    for (const step of s.next_steps) {
      if (step === "DeepCapture") continue;
      const key =
        step === "ReportToAppVendor" && app
          ? `${step}:${app}`
          : step === "UpdateDriver" && driver
            ? `${step}:${driver}`
            : step;
      if (steps.some((known) => known.key === key)) continue;
      // A fall names no driver, so its update step speaks of drivers in general.
      const any = (step === "UpdateDriver" && !driver) || (step === "BackUpNow" && !disk) ? ".any" : "";
      steps.push({
        key,
        text: k(`step.${step}${any}`, { app: app ? appName(app) : "", driver: driver ?? "", disk: disk ? diskName(disk) : "" }),
      });
    }
  }
  return steps;
}

function factTexts(kind: FactKind): Texts {
  const plain = (text: string): Texts => ({ plain: text, figures: null, tech: null });
  if (kind === "FreezeEnded") return plain(k("fact.FreezeEnded"));
  if (kind === "LastWatched") return plain(k("fact.LastWatched"));
  if ("FreezeStarted" in kind) {
    const ms = kind.FreezeStarted.duration_ms;
    return {
      plain: k("fact.FreezeStarted", { duration: formatDuration(ms) }),
      figures: null,
      tech: k("fact.FreezeStarted.tech", { lag: formatMs(ms) }),
    };
  }
  if ("Stutter" in kind) {
    const { count, worst_ms } = kind.Stutter;
    const worst = formatMs(worst_ms);
    return {
      plain: kn("fact.Stutter", count),
      figures: k("fact.Stutter.figures", { worst }),
      tech: kn("fact.Stutter.tech", count, { worst }),
    };
  }
  if ("AppHung" in kind) {
    const exe = kind.AppHung.name;
    return { plain: k("fact.AppHung", { app: appName(exe) }), figures: null, tech: k("fact.AppHung.tech", { exe }) };
  }
  if ("AppRecovered" in kind) return plain(k("fact.AppRecovered", { app: appName(kind.AppRecovered.name) }));
  if ("AppClosedWhileHung" in kind) {
    return plain(k("fact.AppClosedWhileHung", { app: appName(kind.AppClosedWhileHung.name) }));
  }
  if ("AppStillHung" in kind) return plain(k("fact.AppStillHung", { app: appName(kind.AppStillHung.name) }));
  if ("Event" in kind) {
    const { meaning, provider, id } = kind.Event;
    return { plain: k("fact.Event", { event: k(`event.${meaning}`) }), figures: null, tech: k("fact.Event.tech", { provider, id: String(id) }) };
  }
  if ("Crossed" in kind) {
    const { metric, value, threshold, subject } = kind.Crossed;
    const shown = metricValue(metric, value);
    const name = metricName(metric, subject);
    return {
      plain: CROSSED_METRICS.includes(metric)
        ? k(`fact.crossed.${metric}`, { subject: subjectOf(metric, subject) })
        : k("fact.crossed.other", { name }),
      figures: k("fact.crossed.figures", { value: shown }),
      tech: k(FALLING.includes(metric) ? "fact.crossed.tech_below" : "fact.crossed.tech", {
        name,
        threshold: metricValue(metric, threshold),
        value: shown,
      }),
    };
  }
  if ("Busiest" in kind) {
    const { metric, name, value } = kind.Busiest;
    const shown = metricValue(metric, value);
    const app = appName(name);
    return {
      plain: BUSIEST_METRICS.includes(metric) ? k(`fact.busiest.${metric}`, { app }) : k("ev.process.other", { app }),
      figures: PROCESS_METRICS.includes(metric) ? k(`ev.process.${metric}.figures`, { value: shown }) : shown,
      tech: k("fact.busiest.tech", { exe: name, name: k(`metric.${metric}`), value: shown }),
    };
  }
  if ("WentDown" in kind) {
    const crash = kind.WentDown.kind;
    return {
      plain: k(`fact.WentDown.${crashKey(crash)}`),
      figures: null,
      tech:
        typeof crash === "object"
          ? k("fact.WentDown.tech.BlueScreen", { code: bugcheck(crash.BlueScreen.code) })
          : k("fact.WentDown.tech", { kind: crash }),
    };
  }
  if ("EventsDropped" in kind) return plain(k("fact.EventsDropped", { count: formatNumber(kind.EventsDropped.count) }));
  return plain(k(`fact.gap.${kind.Gap.reason}`));
}

/** "0x00000050", as Windows writes a bug check code. */
function bugcheck(code: number): string {
  return `0x${code.toString(16).toUpperCase().padStart(8, "0")}`;
}

function crashKey(kind: CrashKind): "BlueScreen" | Exclude<CrashKind, object> {
  return typeof kind === "object" ? "BlueScreen" : kind;
}

function crashOf(o: Occurrence): CrashTrigger | null {
  return o.triggers.flatMap((tr) => ("Crash" in tr ? [tr.Crash] : []))[0] ?? null;
}

const isMark = (kind: FactKind) =>
  typeof kind !== "string" && ("FreezeStarted" in kind || "AppHung" in kind || "WentDown" in kind);

type FreezeTrigger = Extract<Trigger, { Freeze: unknown }>["Freeze"];
type HangTrigger = Extract<Trigger, { AppHang: unknown }>["AppHang"];
type StutterTrigger = Extract<Trigger, { Stutter: unknown }>["Stutter"];
type CrashTrigger = Extract<Trigger, { Crash: unknown }>["Crash"];

export function freezes(o: Occurrence): FreezeTrigger[] {
  return o.triggers.flatMap((tr) => ("Freeze" in tr ? [tr.Freeze] : []));
}

export function hangs(o: Occurrence): HangTrigger[] {
  return o.triggers.flatMap((tr) => ("AppHang" in tr ? [tr.AppHang] : []));
}

function stutters(o: Occurrence): StutterTrigger[] {
  return o.triggers.flatMap((tr) => ("Stutter" in tr ? [tr.Stutter] : []));
}

function longest<T extends { start_ms: number; end_ms: number }>(spans: T[]): T | null {
  return spans.reduce<T | null>((a, b) => (!a || b.end_ms - b.start_ms > a.end_ms - a.start_ms ? b : a), null);
}

function headline(o: Occurrence, level: DetailLevel): string {
  const kind = k(`class.${o.kind}.${o.severity}`);
  if (o.kind === "Freeze") {
    const freeze = longest(freezes(o));
    const duration = formatDuration(freeze ? freeze.end_ms - freeze.start_ms : o.end_ms - o.start_ms);
    if (level === "Simple") return k("headline.Freeze.Simple", { duration });
    if (level === "Detailed") return k("headline.Detailed", { kind, duration });
    return k("headline.Freeze.Technical", { kind, lag: formatMs(freeze?.lag_ms ?? 0) });
  }
  if (o.kind === "AppHang") {
    const hang = longest(hangs(o));
    const exe = hang?.name ?? "";
    const duration = formatDuration(hang ? hang.end_ms - hang.start_ms : o.end_ms - o.start_ms);
    if (level === "Simple") return k("headline.AppHang.Simple", { app: appName(exe), duration });
    if (level === "Detailed") return k("headline.AppHang.Detailed", { kind, app: appName(exe), duration });
    return k("headline.AppHang.Technical", { exe, pid: String(hang?.pid ?? ""), duration });
  }
  if (o.kind === "Stutter") {
    const all = stutters(o);
    const count = all.reduce((sum, s) => sum + s.count, 0);
    const worst = formatMs(Math.max(0, ...all.map((s) => s.worst_ms)));
    if (level === "Simple") return kn("headline.Stutter.Simple", count);
    if (level === "Detailed") return k("headline.Detailed", { kind, duration: formatDuration(o.end_ms - o.start_ms) });
    return kn("headline.Stutter.Technical", count, { kind, worst });
  }
  const crash = crashOf(o);
  if (crash) {
    const key = crashKey(crash.kind);
    if (level !== "Technical") return k(`headline.PostMortem.${key}`);
    const code = typeof crash.kind === "object" ? bugcheck(crash.kind.BlueScreen.code) : "";
    return k(`headline.PostMortem.Technical.${key}`, { code });
  }
  const event = o.triggers.flatMap((tr) => ("Reported" in tr ? [tr.Reported.event] : []))[0];
  if (!event) return kind;
  if (level === "Simple") return k(`event.${event.meaning}`);
  if (level === "Detailed") return k("headline.Reported.Detailed", { kind, event: k(`event.${event.meaning}`) });
  return k("headline.Reported.Technical", { provider: event.provider, id: String(event.id), channel: event.channel });
}

function figureOf(o: Occurrence): { value: string; caption: string } {
  const crash = crashOf(o);
  if (crash) return { value: formatTime(crash.down_ms), caption: k("figure.PostMortem") };
  const span =
    o.kind === "Freeze" ? longest(freezes(o)) : o.kind === "AppHang" ? longest(hangs(o)) : o;
  if (o.kind === "Reported" || !span) {
    return { value: formatTime(o.start_ms), caption: k("figure.Reported") };
  }
  return { value: formatDuration(span.end_ms - span.start_ms), caption: k("figure.duration") };
}

export function buildReport(dto: OccurrenceDto, level: DetailLevel, fromDump: Step[] | null = null): ReportView {
  const [first, ...rest] = dto.suspects;
  const main = first ?? { cause: "Unclear", confidence: "Low", evidence: [], next_steps: [] };
  const figure = figureOf(dto);
  // A fall is named by when it went down, not by the last second stored before it.
  const at = crashOf(dto)?.down_ms ?? dto.start_ms;
  const when = `${formatDay(at)}, ${formatClock(at)}`;
  return {
    crumb: sameDay(at, Date.now()) ? formatTime(at) : `${formatDayShort(at)}, ${formatTime(at)}`,
    title: headline(dto, level),
    meta: [when, k(`class.${dto.kind}.${dto.severity}`), dto.sensitive ? k("sensitive") : ""]
      .filter(Boolean)
      .join(" · "),
    when: [when, dto.sensitive ? k("sensitive") : ""].filter(Boolean).join(" · "),
    severity: k(`class.${dto.kind}.${dto.severity}`),
    where: shortCause(main.cause),
    figure: figure.value,
    figureCaption: figure.caption,
    tone: toneOf(dto),
    cause: causeKind(main.cause),
    main: suspectView(main, dto, level),
    others: rest.map((s) => suspectView(s, dto, level)),
    summary: summaryOf(main.cause, main.confidence, main.evidence),
    note: noteOf(dto.suspects),
    steps: stepsOf(dto.suspects, fromDump),
    facts: dto.facts.map((f) => ({
      time: formatClock(f.at_ms),
      datetime: new Date(f.at_ms).toISOString(),
      line: lineFor(level, factTexts(f.kind)),
      mark: isMark(f.kind),
    })),
  };
}

/** The report as Markdown, plain words with the counters beside them, for a forum or an issue. */
export function reportMarkdown(dto: OccurrenceDto, fromDump: Step[] | null = null): string {
  const lines: string[] = [];
  const figure = figureOf(dto);
  const kind = k(`class.${dto.kind}.${dto.severity}`);
  const head = [formatStamp(dto.start_ms), kind];
  if (dto.kind !== "Reported") head.push(`${figure.value} ${figure.caption}`);
  if (dto.sensitive) head.push(k("sensitive"));
  lines.push(`## ${headline(dto, "Simple")}`, "", head.join(" · "), "");
  const main = dto.suspects[0];
  const fall = crashOf(dto) ? ".PostMortem" : "";
  if (main) {
    lines.push(`**${k("cause.title")}** (${k(`confidence.${main.confidence}`)})`, "");
    lines.push(summaryOf(main.cause, main.confidence, main.evidence), "");
  }
  const notes: string[] = [];
  if (dto.truncated) notes.push(k(`chart.truncated${fall}`));
  if (!fall && dto.window.samples.length > 0 && freezes(dto).length > 0) notes.push(k("md.blank"));
  if (dto.tests.length) notes.push(k("during_test", { test: k(`test.${dto.tests[0].kind}`) }));
  if (dto.capture) notes.push(coveredLine(dto.capture));
  else {
    const suggestion = noteOf(dto.suspects);
    if (suggestion) notes.push(suggestion);
  }
  for (const note of notes) lines.push(`> ${note}`, "");

  lines.push(`### ${k("md.suspects")}`, "");
  dto.suspects.forEach((s, i) => {
    const { name, parts } = causeName(s.cause, dto);
    const sub = parts.filter(Boolean).join(" · ");
    lines.push(`${i + 1}. **${name}**${sub ? ` (${sub})` : ""} · ${k(`confidence.${s.confidence}`)}`);
    for (const e of s.evidence) {
      const texts = evidenceTexts(e, causeKind(s.cause));
      const counters = texts.tech ?? texts.figures;
      lines.push(`   - ${texts.plain}${counters ? ` — ${counters}` : ""}`);
    }
  });

  lines.push("", `### ${k("md.facts")}`, "");
  for (const f of dto.facts) {
    const texts = factTexts(f.kind);
    const counters = texts.tech ?? texts.figures;
    lines.push(`- ${formatClock(f.at_ms)} — ${texts.plain}${counters ? ` (${counters})` : ""}`);
  }

  if (dto.window.events.length) {
    lines.push("", `### ${k("md.events")}`, "");
    for (const e of dto.window.events) {
      lines.push(`- ${formatClock(e.ts_ms)} · ${e.provider} ${e.id} (${e.channel}): ${k(`event.${e.meaning}`)}`);
    }
  }

  lines.push("", `### ${k("md.steps")}`, "");
  const steps = stepsOf(dto.suspects, fromDump);
  if (steps.length) for (const step of steps) lines.push(`- ${step.text}`);
  else lines.push(k("todo.none"));
  lines.push("", `_${k("md.footer")}_`);
  return lines.join("\n");
}
