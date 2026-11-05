import { formatDuration, formatPercent, formatTime } from "../../lib/format";
import { t } from "../../lib/i18n.svelte";
import type { Columns, DetailLevel, Episode, EpisodeDetailDto, ProcessSample } from "../../lib/types";
import { type Band, type ChartView, type LaneKey, lanesOf } from "../occurrence/chart";
import { formatBytes, formatMs, formatRate } from "../occurrence/format";
import { episodeDisk, episodeKey, kindName } from "./text";

// What the detail of a pressure episode shows: its numbers, its chart, who used what and what to do.

const KEY = "ui.today.period";

type PeriodKind = ReturnType<typeof episodeKey>;
type Measure = "cpu" | "mem" | "io";

/** The lanes each kind shows first; Detailed adds processor and memory, Technical every lane. */
const LANES: Record<PeriodKind, LaneKey[]> = {
  CpuSaturated: ["cpu", "mem"],
  MemoryHigh: ["mem", "cpu"],
  DiskBusy: ["disk", "latency"],
};

/** The column that crossed the threshold. */
const METRIC: Record<PeriodKind, keyof Omit<Columns, "t" | "state">> = {
  CpuSaturated: "cpu",
  MemoryHigh: "mem_used_pct",
  DiskBusy: "disk_active_max",
};

/** The program table's two measures, the first one ranking it. */
const MEASURES: Record<PeriodKind, [Measure, Measure]> = {
  CpuSaturated: ["cpu", "mem"],
  MemoryHigh: ["mem", "cpu"],
  DiskBusy: ["io", "cpu"],
};

const PROGRAMS_SHOWN = 5;

export function laneKeysOf(e: Episode, level: DetailLevel): LaneKey[] {
  const first = LANES[episodeKey(e)];
  if (level === "Simple") return first;
  const more: LaneKey[] = level === "Detailed" ? ["cpu", "mem"] : ["cpu", "mem", "disk", "latency", "gpu", "lag"];
  return [...new Set([...first, ...more])];
}

/** The chart: seconds from the start of the episode, the episode and what stalled in it as bands. */
export function periodView(e: Episode, dto: EpisodeDetailDto, level: DetailLevel): ChartView {
  const origin = e.start_ms / 1000;
  const disk = episodeDisk(e);
  const names: Partial<Record<LaneKey, string>> = disk
    ? { disk: t(`${KEY}.lane.disk`, { disk }), latency: t(`${KEY}.lane.latency`, { disk }) }
    : {};
  const { lanes, summary } = lanesOf(dto.chart, laneKeysOf(e, level), names);
  const rel = (ms: number) => ms / 1000 - origin;
  const bands: Band[] = [
    {
      from: 0,
      to: rel(e.end_ms),
      tone: "press",
      label: t(`${KEY}.band`, { duration: formatDuration(e.end_ms - e.start_ms) }),
    },
    ...dto.occurrences.map((o) => ({
      from: rel(o.start_ms),
      to: rel(Math.max(o.end_ms, o.start_ms + 1000)),
      tone: "freeze" as const,
      label: `${kindName(o)} · ${formatDuration(o.end_ms - o.start_ms)}`,
    })),
  ];
  for (const b of bands) {
    summary.push(
      t("ui.occurrence.chart.summary.band", {
        label: b.label,
        from: formatTime((origin + b.from) * 1000),
        to: formatTime((origin + b.to) * 1000),
      }),
    );
  }
  return { x: dto.chart.t.map((s) => s - origin), originMs: e.start_ms, lanes, bands, markers: [], summary };
}

/** Known values of `column` in the points that overlap the episode. */
function inside(e: Episode, dto: EpisodeDetailDto, column: keyof Omit<Columns, "t" | "state">): number[] {
  return dto.chart[column].filter((v, i): v is number => {
    const ms = dto.chart.t[i] * 1000;
    return v !== null && ms + dto.step_ms > e.start_ms && ms <= e.end_ms;
  });
}

const mean = (values: number[]) => values.reduce((a, b) => a + b, 0) / values.length;

export interface Stat {
  label: string;
  value: string;
}

/** The numbers above the chart: when, the peak, the mean of the metric and one measure beside it. */
export function periodStats(e: Episode, dto: EpisodeDetailDto): Stat[] {
  const kind = episodeKey(e);
  const metric = inside(e, dto, METRIC[kind]);
  const stats: Stat[] = [
    { label: t(`${KEY}.stat.period`), value: `${formatTime(e.start_ms)} – ${formatTime(e.end_ms)}` },
    { label: t(`${KEY}.stat.peak`), value: formatPercent(Math.max(e.peak, ...metric)) },
  ];
  if (metric.length) stats.push({ label: t(`${KEY}.stat.mean`), value: formatPercent(mean(metric)) });
  if (kind === "CpuSaturated") {
    const memory = inside(e, dto, "mem_used_pct");
    if (memory.length) stats.push({ label: t(`${KEY}.stat.memory`), value: formatPercent(Math.max(...memory)) });
  } else if (kind === "MemoryHigh") {
    const cpu = inside(e, dto, "cpu");
    if (cpu.length) stats.push({ label: t(`${KEY}.stat.cpu`), value: formatPercent(mean(cpu)) });
  } else {
    const latency = inside(e, dto, "disk_latency_max");
    if (latency.length) stats.push({ label: t(`${KEY}.stat.latency`), value: formatMs(Math.max(...latency)) });
  }
  return stats;
}

function measured(p: ProcessSample, measure: Measure): number {
  return measure === "cpu" ? p.cpu_pct : measure === "mem" ? p.private_bytes : p.read_bps + p.write_bps;
}

function shown(value: number, measure: Measure): string {
  if (measure === "cpu") return formatPercent(value, value < 10 ? 1 : 0);
  return measure === "mem" ? formatBytes(value) : formatRate(value);
}

export interface ProgramTable {
  columns: [string, string];
  rows: { pid: number; name: string; values: [string, string] }[];
}

/** The programs that used most of what ran out, heaviest first. */
export function periodPrograms(e: Episode, dto: EpisodeDetailDto): ProgramTable {
  const [first, second] = MEASURES[episodeKey(e)];
  const rows = dto.programs
    .filter((p) => measured(p, first) > 0)
    .sort((a, b) => measured(b, first) - measured(a, first))
    .slice(0, PROGRAMS_SHOWN)
    .map((p) => ({
      pid: p.pid,
      name: p.name,
      values: [shown(measured(p, first), first), shown(measured(p, second), second)] as [string, string],
    }));
  return { columns: [t(`${KEY}.col.${first}`), t(`${KEY}.col.${second}`)], rows };
}

/** What to do; a step that names the Now tab links to it. */
export function periodSteps(e: Episode, byTest: boolean): { key: string; text: string }[] {
  if (byTest) return [{ key: "by_test", text: t(`${KEY}.steps.by_test`) }];
  const kind = episodeKey(e);
  return [
    { key: "first", text: t(`${KEY}.steps.${kind}.1`) },
    { key: kind === "MemoryHigh" ? "second" : "SeeNow", text: t(`${KEY}.steps.${kind}.2`) },
  ];
}
