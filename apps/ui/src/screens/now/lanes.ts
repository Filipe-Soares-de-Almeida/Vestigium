import { formatNumber, formatPercent } from "../../lib/format";
import { lang } from "../../lib/i18n.svelte";
import type { DiskLoad, Lanes, LivePoint, PcState, ProcessSample, Reading } from "../../lib/types";

export type LaneKey = keyof Lanes;

/** The drive letters of the busiest disk in a second, such as "C:"; `null` when it holds none. */
export function busiestDisk(disks: readonly DiskLoad[]): string | null {
  const top = disks.reduce<DiskLoad | null>((a, d) => (a === null || d.active_pct > a.active_pct ? d : a), null);
  const letters = top?.name.split(" ").filter((part) => /^[A-Z]:$/i.test(part)) ?? [];
  return letters.length ? letters.join(" ") : null;
}

/** The four lanes of the live chart, top to bottom, with the reading each one draws. */
export const LANES: readonly { key: LaneKey; value: (reading: Reading) => number | null }[] = [
  { key: "cpu", value: (r) => r.cpu_pct },
  { key: "memory", value: (r) => r.mem_used_pct },
  { key: "disk", value: (r) => r.disk_active_pct },
  { key: "gpu", value: (r) => r.gpu_pct },
];

/** The token each state is drawn in. */
export const STATE_TOKEN: Record<PcState, string> = {
  calm: "calm",
  busy: "busy",
  pressure: "press",
  suspended: "susp",
};

const RANK: Record<PcState, number> = { suspended: 0, calm: 1, busy: 2, pressure: 3 };

/** The lane that weighs most: the strongest state, then the highest value. */
export function heaviest(point: LivePoint): LaneKey {
  let best = LANES[0];
  for (const lane of LANES.slice(1)) {
    const a = RANK[point.lanes[lane.key]];
    const b = RANK[point.lanes[best.key]];
    if (a > b || (a === b && (lane.value(point.reading) ?? 0) > (best.value(point.reading) ?? 0))) {
      best = lane;
    }
  }
  return best.key;
}

/** The process using most of what a lane measures. */
export function busiest(entries: ProcessSample[], lane: LaneKey): ProcessSample | null {
  const weight = (p: ProcessSample) =>
    lane === "cpu"
      ? p.cpu_pct
      : lane === "memory"
        ? p.private_bytes
        : lane === "disk"
          ? p.read_bps + p.write_bps
          : p.gpu_pct;
  const top = entries.reduce<ProcessSample | null>(
    (best, p) => (best === null || weight(p) > weight(best) ? p : best),
    null,
  );
  return top && weight(top) > 0 ? top : null;
}

/** A lane's value as the chart labels show it. */
export function laneValue(value: number | null): string | null {
  return value === null ? null : formatPercent(value);
}

/** "3,9 GB" or "610 MB". */
export function formatBytes(bytes: number): string {
  const gb = bytes / 2 ** 30;
  return gb >= 1
    ? new Intl.NumberFormat(lang(), { style: "unit", unit: "gigabyte", maximumFractionDigits: 1 }).format(gb)
    : new Intl.NumberFormat(lang(), { style: "unit", unit: "megabyte", maximumFractionDigits: 0 }).format(
        bytes / 2 ** 20,
      );
}

/** "12,5 MB/s". */
export function formatRate(bytesPerSecond: number): string {
  return new Intl.NumberFormat(lang(), {
    style: "unit",
    unit: "megabyte-per-second",
    maximumFractionDigits: 1,
  }).format(bytesPerSecond / 2 ** 20);
}

/** "1,2 ms" below a second, "1,4 s" above. */
export function formatLatency(ms: number): string {
  return ms >= 1000 ? `${formatNumber(ms / 1000, 1)} s` : `${formatNumber(ms, 1)} ms`;
}
