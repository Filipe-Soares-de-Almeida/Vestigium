import { formatDuration, formatPercent } from "../../lib/format";
import { t, tn } from "../../lib/i18n.svelte";
import type { DetailLevel, EpisodeKind, LivePoint, NowFrame, ProcessSample } from "../../lib/types";
import type { Hung } from "./live.svelte";
import { busiest, formatBytes, formatLatency, formatRate, heaviest, type LaneKey } from "./lanes";

export interface Moment {
  point: LivePoint;
  frame: NowFrame;
  hung: Hung[];
  nowMs: number;
}

/** What changes the answer: the state, the episodes open and the windows not responding. */
export function momentKey({ point, frame, hung }: Moment): string {
  return [point.state, JSON.stringify(frame.pressure), hung.map((h) => h.pid).join(",")].join("|");
}

/** The answer to "what is weighing now", at one detail level. */
export function sentence(moment: Moment, level: DetailLevel): string {
  if (level === "Technical") return technical(moment);
  const { point, frame, hung } = moment;
  const parts: string[] = [];
  if (hung.length > 1) parts.push(tn("ui.now.lede.hung.many", hung.length));
  else if (hung.length === 1) {
    parts.push(
      level === "Simple"
        ? t("ui.now.lede.hung.one", { name: hung[0].name })
        : t("ui.now.lede.hung.for", { name: hung[0].name, duration: since(hung[0], moment.nowMs) }),
    );
  }
  if (point.state === "calm" || point.state === "suspended") {
    parts.push(t("ui.now.lede.calm"));
  } else {
    const lane = heaviest(point);
    const state = point.state === "pressure" ? "pressure" : "busy";
    const who = busiest(frame.top?.entries ?? [], lane);
    if (level === "Simple" && who) {
      parts.push(t(`ui.now.lede.${state}.${lane}_by`, { name: who.name }));
    } else {
      parts.push(t(`ui.now.lede.${state}.${lane}`));
    }
    if (level === "Simple") parts.push(t(`ui.now.lede.${state}.tail`));
    else if (who) parts.push(t("ui.now.lede.by_value", { name: who.name, value: share(who, lane) }));
  }
  if (level === "Detailed") parts.push(numbers(point));
  return parts.join(" ");
}

function numbers({ reading }: LivePoint): string {
  const args = {
    cpu: formatPercent(reading.cpu_pct),
    memory: reading.mem_used_pct === null ? "—" : formatPercent(reading.mem_used_pct),
    disk: formatPercent(reading.disk_active_pct),
  };
  return reading.gpu_pct === null
    ? t("ui.now.lede.numbers", args)
    : t("ui.now.lede.numbers_gpu", { ...args, gpu: formatPercent(reading.gpu_pct) });
}

function share(process: ProcessSample, lane: LaneKey): string {
  if (lane === "memory") return formatBytes(process.private_bytes);
  if (lane === "disk") return formatRate(process.read_bps + process.write_bps);
  return formatPercent(lane === "gpu" ? process.gpu_pct : process.cpu_pct, 1);
}

function since(hung: Hung, nowMs: number): string {
  return formatDuration(Math.max(0, nowMs - hung.sinceMs));
}

/** The counters behind the answer, one after another. */
function technical({ point, frame, hung, nowMs }: Moment): string {
  const { reading } = point;
  const parts = [t("ui.now.tech.cpu", { value: formatPercent(reading.cpu_pct, 1) })];
  const sample = frame.sample;
  if (reading.mem_used_pct !== null && sample) {
    parts.push(
      t("ui.now.tech.memory", {
        value: formatPercent(reading.mem_used_pct),
        used: formatBytes(sample.mem_total_bytes - sample.mem_avail_bytes),
        total: formatBytes(sample.mem_total_bytes),
      }),
    );
  }
  parts.push(
    t("ui.now.tech.disk", {
      value: formatPercent(reading.disk_active_pct),
      latency: formatLatency(reading.disk_latency_ms),
    }),
  );
  if (reading.gpu_pct !== null) parts.push(t("ui.now.tech.gpu", { value: formatPercent(reading.gpu_pct) }));
  parts.push(t("ui.now.tech.probe", { value: formatLatency(reading.probe_lag_ms) }));
  for (const kind of frame.pressure) parts.push(episode(kind));
  for (const window of hung) {
    parts.push(t("ui.now.tech.hung", { name: window.name, duration: since(window, nowMs) }));
  }
  return t("ui.now.tech.line", { parts: parts.join(" · ") });
}

function episode(kind: EpisodeKind): string {
  if (typeof kind === "string") return t(`ui.now.tech.episode.${kind}`);
  if ("DiskBusy" in kind) return t("ui.now.tech.episode.DiskBusy", { disk: kind.DiskBusy.disk });
  return "";
}
