import { formatPercent, formatTime } from "../../lib/format";
import { t } from "../../lib/i18n.svelte";
import type { Columns, Episode, EpisodeDetailDto, ProcessSample } from "../../lib/types";
import { formatBytes, formatMs, formatRate } from "../occurrence/format";
import { episodeKey } from "./text";

// What the detail of a pressure episode shows: its numbers, who used what and what to do.

const KEY = "ui.today.period";

type PeriodKind = ReturnType<typeof episodeKey>;
type Measure = "cpu" | "mem" | "io";

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
