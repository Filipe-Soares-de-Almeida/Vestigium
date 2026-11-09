import { formatClock, formatDuration, formatPercent } from "../../lib/format";
import { t } from "../../lib/i18n.svelte";
import type { Columns, DetailLevel, OccurrenceDto } from "../../lib/types";
import { appName, formatMs } from "./format";
import { type CauseKind, freezes, hangs } from "./report";

// What the window chart draws: which lanes, the spans when something stopped answering, the events.

export type LaneKey = "cpu" | "mem" | "disk" | "latency" | "gpu" | "lag";
export type LaneTone = "calm" | "busy" | "press" | "freeze";

export interface Lane {
  key: LaneKey;
  /** The lane's name; `label` adds the scale when it is not linear. */
  name: string;
  label: string;
  values: (number | null)[];
  /** Waits run from a millisecond to seconds, so they are drawn on a log scale. */
  log: boolean;
  tone: LaneTone;
  peak: string;
  format: (value: number) => string;
}

export interface Band {
  /** Seconds from the start of the occurrence. */
  from: number;
  to: number;
  tone: "freeze" | "press";
  /** Nothing could be read in it: the PC did not respond or was down, so the lines break. */
  blank: boolean;
  label: string;
}

export interface Marker {
  at: number;
  label: string;
}

export interface ChartView {
  /** Seconds from the start of the occurrence, one per point. */
  x: number[];
  originMs: number;
  lanes: Lane[];
  bands: Band[];
  markers: Marker[];
  summary: string[];
}

const COLUMN: Record<LaneKey, keyof Omit<Columns, "t" | "state">> = {
  cpu: "cpu",
  mem: "mem_used_pct",
  disk: "disk_active_max",
  latency: "disk_latency_max",
  gpu: "gpu",
  lag: "probe_lag",
};

/** The lanes that show each cause first. */
const RELEVANT: Record<CauseKind, LaneKey[]> = {
  Storage: ["latency", "disk"],
  DiskIo: ["disk", "latency"],
  Memory: ["mem"],
  Cpu: ["cpu"],
  Driver: ["cpu"],
  GpuDriver: ["gpu", "cpu"],
  Throttling: ["cpu"],
  App: ["cpu", "disk"],
  Unclear: ["cpu", "mem"],
  DriverModule: ["cpu"],
  Frames: ["gpu", "cpu"],
  BlueScreen: ["cpu", "mem", "disk", "lag"],
  Unresponsive: ["cpu", "mem", "disk", "lag"],
  SleepTransition: ["cpu", "disk"],
  PowerCut: ["cpu", "mem", "lag"],
  HardwareFault: ["cpu", "gpu"],
};

const TONES: LaneTone[] = ["calm", "busy", "press", "freeze"];
const ALL: LaneKey[] = ["cpu", "mem", "disk", "latency", "gpu", "lag"];

/** Simple shows what points at the cause, Detailed adds processor and memory, Technical shows every lane. */
export function laneKeys(level: DetailLevel, cause: CauseKind): LaneKey[] {
  const relevant = RELEVANT[cause];
  if (level === "Simple") return relevant.slice(0, 2);
  const keys = level === "Detailed" ? [...relevant, "cpu", "mem"] : [...relevant, ...ALL];
  return [...new Set(keys as LaneKey[])];
}

const isMs = (key: LaneKey) => key === "latency" || key === "lag";

/** The lanes of `keys` that hold a reading, with a line each for screen readers; `names` renames lanes. */
export function lanesOf(
  chart: Columns,
  keys: LaneKey[],
  names: Partial<Record<LaneKey, string>> = {},
): { lanes: Lane[]; summary: string[] } {
  const lanes: Lane[] = [];
  const summary: string[] = [];
  for (const key of keys) {
    const values = chart[COLUMN[key]];
    const known = values.flatMap((v, i) => (v === null ? [] : [[v, i] as const]));
    if (!known.length) continue;
    const format = isMs(key) ? (v: number) => formatMs(v) : (v: number) => formatPercent(v);
    const [max, at] = known.reduce((a, b) => (b[0] > a[0] ? b : a));
    const min = Math.min(...known.map(([v]) => v));
    const log = isMs(key);
    const label = names[key] ?? t(`ui.occurrence.lane.${key}`);
    lanes.push({
      key,
      name: label,
      label: log ? `${label} · ${t("ui.occurrence.chart.log")}` : label,
      values,
      log,
      tone: TONES[lanes.length % TONES.length],
      peak: t("ui.occurrence.chart.peak", { value: format(max) }),
      format,
    });
    summary.push(
      t("ui.occurrence.chart.summary.lane", {
        lane: label,
        min: format(min),
        max: format(max),
        time: formatClock(chart.t[at] * 1000),
      }),
    );
  }
  return { lanes, summary };
}

export function chartView(dto: OccurrenceDto, level: DetailLevel, cause: CauseKind): ChartView {
  const origin = dto.start_ms / 1000;
  const x = dto.chart.t.map((s) => s - origin);
  const rel = (ms: number) => ms / 1000 - origin;
  const { lanes, summary } = lanesOf(dto.chart, laneKeys(level, cause));

  const bands: Band[] = [];
  for (const f of freezes(dto)) {
    bands.push({
      from: rel(f.start_ms),
      to: rel(f.end_ms),
      tone: "freeze",
      blank: true,
      label: t("ui.occurrence.chart.band.Freeze", { duration: formatDuration(f.end_ms - f.start_ms) }),
    });
  }
  for (const h of hangs(dto)) {
    bands.push({
      from: rel(h.start_ms),
      to: rel(h.end_ms),
      tone: "freeze",
      blank: false,
      label: t("ui.occurrence.chart.band.AppHang", { app: appName(h.name) }),
    });
  }
  for (const tr of dto.triggers) {
    if ("Crash" in tr) {
      bands.push({
        from: rel(tr.Crash.last_ms),
        to: rel(Math.max(tr.Crash.down_ms, tr.Crash.last_ms + 1000)),
        tone: "freeze",
        blank: true,
        label: t("ui.occurrence.chart.band.PostMortem"),
      });
    }
    if ("Stutter" in tr) {
      bands.push({
        from: rel(tr.Stutter.start_ms),
        to: rel(tr.Stutter.end_ms),
        tone: "press",
        blank: false,
        label: t("ui.occurrence.chart.band.Stutter"),
      });
    }
  }
  for (const b of bands) {
    summary.push(
      t("ui.occurrence.chart.summary.band", {
        label: b.label,
        from: formatClock((origin + b.from) * 1000),
        to: formatClock((origin + b.to) * 1000),
      }),
    );
  }

  // Marks only the events inside the readings, so a fall's next-boot events stay off the axis.
  const end = x.length ? x[x.length - 1] : 0;
  const shown = dto.window.events.filter((e) => rel(e.ts_ms) >= (x[0] ?? 0) && rel(e.ts_ms) <= end);
  const markers = shown.map((e) => ({
    at: rel(e.ts_ms),
    label: t(`ui.occurrence.event.${e.meaning}`),
  }));
  for (const [i, m] of markers.entries()) {
    summary.push(t("ui.occurrence.chart.summary.event", { event: m.label, time: formatClock(shown[i].ts_ms) }));
  }
  return { x, originMs: dto.start_ms, lanes, bands, markers, summary };
}
