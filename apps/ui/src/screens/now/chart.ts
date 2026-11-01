import "uplot/dist/uPlot.min.css";

import uPlot from "uplot";

import type { LivePoint } from "../../lib/types";
import { LANES, STATE_TOKEN } from "./lanes";

/** Height of one lane, and of the band inside it where 0 to 100% is drawn. */
export const LANE_H = 56;
const BAND_TOP = 9;
const BAND_H = 38;
/** Room on the right for the dot of the latest value. */
export const RIGHT = 7;
export const HEIGHT = LANES.length * LANE_H;
const SPAN_S = 600;
/** A longer step than this between seconds is a gap, and the line breaks there. */
const GAP_S = 2.5;

export interface Built {
  /** Seconds, then one column per lane. */
  data: uPlot.AlignedData;
  /** The second behind each column entry; `null` where a gap breaks the lines. */
  rows: (LivePoint | null)[];
}

export function build(points: LivePoint[]): Built {
  const xs: number[] = [];
  const rows: (LivePoint | null)[] = [];
  for (const point of points) {
    const x = point.reading.ts_ms / 1000;
    const prev = xs.at(-1);
    if (prev !== undefined && x - prev > GAP_S) {
      xs.push((prev + x) / 2);
      rows.push(null);
    }
    xs.push(x);
    rows.push(point);
  }
  const lanes = LANES.map((lane) =>
    rows.map((row) => {
      const value = row ? lane.value(row.reading) : null;
      return value === null ? null : Math.min(100, Math.max(0, value));
    }),
  );
  return { data: [xs, ...lanes], rows };
}

type Colors = Record<"busy" | "card" | "stroke" | "stroke2" | "calm" | "press" | "susp", string>;

function readColors(): Colors {
  const style = getComputedStyle(document.documentElement);
  const get = (name: string) => style.getPropertyValue(`--${name}`).trim();
  return {
    busy: get("busy"),
    card: get("card"),
    stroke: get("stroke"),
    stroke2: get("stroke2"),
    calm: get("calm"),
    press: get("press"),
    susp: get("susp"),
  };
}

/** A token colour at `alpha` opacity; the tokens are six-digit hex. */
function faded(hex: string, alpha: number): string {
  const byte = Math.round(alpha * 255)
    .toString(16)
    .padStart(2, "0");
  return /^#[0-9a-f]{6}$/i.test(hex) ? hex + byte : hex;
}

export interface LaneChart {
  set(built: Built): void;
  /** Moves the readout `secondsAgo` before the latest second, or takes it away with `null`. */
  readAt(secondsAgo: number | null): void;
  destroy(): void;
}

/**
 * Draws the four lanes in one plot, each on its own scale so it fills its band; `onread` gets the
 * column under the readout and its left edge in CSS pixels, or `null` when nothing is read.
 */
export function laneChart(
  target: HTMLElement,
  onread: (idx: number | null, left: number) => void,
): LaneChart {
  let colors = readColors();
  let rows: (LivePoint | null)[] = [];
  const perPct = BAND_H / 100;

  const scales: uPlot.Scales = {
    x: {
      time: false,
      range: (_u, _min, max) => (max == null ? [0, SPAN_S] : [max - SPAN_S, max]),
    },
  };
  LANES.forEach((_, i) => {
    const min = (i * LANE_H + BAND_TOP + BAND_H - HEIGHT) / perPct;
    scales[`l${i}`] = { auto: false, range: [min, min + HEIGHT / perPct] };
  });

  const options: uPlot.Options = {
    width: Math.max(1, target.clientWidth),
    height: HEIGHT,
    padding: [0, RIGHT, 0, 0],
    legend: { show: false },
    select: { show: false, left: 0, top: 0, width: 0, height: 0 },
    scales,
    axes: [{ show: false }, ...LANES.map((_, i) => ({ show: false, scale: `l${i}` }))],
    series: [
      {},
      ...LANES.map((_, i) => ({
        scale: `l${i}`,
        width: 2,
        stroke: () => colors.busy,
        fill: () => faded(colors.busy, 0.16),
        fillTo: 0,
        spanGaps: false,
        points: { show: false },
      })),
    ],
    cursor: {
      y: false,
      drag: { x: false, y: false, setScale: false },
      points: { size: 9, width: 2, fill: () => colors.card, stroke: () => colors.busy },
    },
    hooks: {
      drawAxes: [
        (u) => {
          const { ctx } = u;
          const ratio = devicePixelRatio;
          const { left, width } = u.bbox;
          ctx.save();
          ctx.lineWidth = ratio;
          ctx.strokeStyle = colors.stroke2;
          for (let i = 1; i < LANES.length; i++) {
            const y = Math.round(i * LANE_H * ratio) + 0.5;
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(ctx.canvas.width, y);
            ctx.stroke();
          }
          ctx.strokeStyle = colors.stroke;
          ctx.setLineDash([2 * ratio, 4 * ratio]);
          LANES.forEach((_, i) => {
            const y = Math.round(u.valToPos(0, `l${i}`, true)) + 0.5;
            ctx.beginPath();
            ctx.moveTo(left, y);
            ctx.lineTo(left + width, y);
            ctx.stroke();
          });
          ctx.restore();
        },
      ],
      draw: [
        (u) => {
          const latest = rows.at(-1);
          const xs = u.data[0];
          if (!latest || xs.length === 0) return;
          const { ctx } = u;
          const ratio = devicePixelRatio;
          const x = u.valToPos(xs[xs.length - 1], "x", true);
          ctx.save();
          LANES.forEach((lane, i) => {
            const value = u.data[i + 1][xs.length - 1];
            if (value == null) return;
            ctx.beginPath();
            ctx.arc(x, u.valToPos(value, `l${i}`, true), 4.5 * ratio, 0, 2 * Math.PI);
            ctx.fillStyle = colors[STATE_TOKEN[latest.lanes[lane.key]] as keyof Colors];
            ctx.fill();
            ctx.lineWidth = 2 * ratio;
            ctx.strokeStyle = colors.card;
            ctx.stroke();
          });
          ctx.restore();
        },
      ],
      setCursor: [(u) => onread(u.cursor.idx ?? null, u.cursor.left ?? -1)],
    },
  };

  const plot = new uPlot(options, [[]], target);
  const theme = new MutationObserver(() => {
    colors = readColors();
    plot.redraw(false);
  });
  theme.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme", "data-scheme"],
  });
  const size = new ResizeObserver(() =>
    plot.setSize({ width: Math.max(1, target.clientWidth), height: HEIGHT }),
  );
  size.observe(target);

  return {
    set(built) {
      rows = built.rows;
      plot.setData(built.data);
    },
    readAt(secondsAgo) {
      const xs = plot.data[0];
      if (secondsAgo === null || xs.length === 0) {
        plot.setCursor({ left: -10, top: -10 });
        return;
      }
      const left = plot.valToPos(xs[xs.length - 1] - secondsAgo, "x");
      plot.setCursor({ left, top: HEIGHT / 2 });
    },
    destroy() {
      theme.disconnect();
      size.disconnect();
      plot.destroy();
    },
  };
}
