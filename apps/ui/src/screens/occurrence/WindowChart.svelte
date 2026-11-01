<script lang="ts">
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";

  import { formatClock } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { ChartView, Lane } from "./chart";
  import { formatOffset } from "./format";

  let { view }: { view: ChartView } = $props();

  const PLOT_H = 52;
  /** Axis steps in seconds, and the room one label needs before the next step is used. */
  const TICK_STEPS = [15, 30, 60];
  const TICK_ROOM_PX = 56;
  const uid = `occ-chart-${Math.random().toString(36).slice(2, 8)}`;

  let wrap: HTMLDivElement;
  let nodes = $state<HTMLDivElement[]>([]);
  /** The point the readout shows, from the mouse or the keyboard; the slider speaks it. */
  let cursor = $state<number | null>(null);
  let plots: uPlot[] = [];
  let width = $state(0);

  const first = $derived(view.x[0] ?? 0);
  const last = $derived(view.x.at(-1) ?? 0);
  /** The first point at or after the start of the occurrence, where the keyboard starts. */
  const moment = $derived(Math.max(0, view.x.findIndex((s) => s >= 0)));
  const pct = (s: number) => ((s - first) / Math.max(1, last - first)) * 100;
  const ticks = $derived.by(() => {
    const perSecond = width / Math.max(1, last - first);
    const step = TICK_STEPS.find((s) => s * perSecond >= TICK_ROOM_PX) ?? TICK_STEPS.at(-1)!;
    const out: number[] = [];
    for (let s = Math.ceil(first / step) * step; s <= last; s += step) out.push(s);
    return out;
  });

  function token(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  /** A token's hex colour at `alpha`, since a canvas gradient takes no CSS colour mixing. */
  function faded(hex: string, alpha: number): string {
    const digits = hex.replace("#", "");
    const full = digits.length === 3 ? [...digits].map((c) => c + c).join("") : digits.slice(0, 6);
    const n = Number.parseInt(full, 16);
    return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
  }

  function options(lane: Lane, width: number, x0: number, x1: number): uPlot.Options {
    const peak = Math.max(1, ...lane.values.map((v) => v ?? 0));
    return {
      width,
      height: PLOT_H,
      padding: [6, 0, 2, 0],
      legend: { show: false },
      select: { show: false, left: 0, top: 0, width: 0, height: 0 },
      scales: {
        x: { time: false, range: [x0, x1] },
        y: lane.log ? { distr: 3, range: [1, Math.max(1000, peak)] } : { range: [0, 100] },
      },
      axes: [{ show: false }, { show: false }],
      cursor: {
        sync: { key: uid },
        y: false,
        drag: { x: false, y: false, setScale: false },
        points: { size: 7, width: 2, stroke: () => token("--ink"), fill: () => token("--card") },
      },
      series: [
        {},
        {
          stroke: () => token(`--${lane.tone}`),
          width: 2,
          fill: (u) => {
            const color = token(`--${lane.tone}`);
            const fill = u.ctx.createLinearGradient(0, u.bbox.top, 0, u.bbox.top + u.bbox.height);
            fill.addColorStop(0, faded(color, 0.42));
            fill.addColorStop(1, faded(color, 0));
            return fill;
          },
          spanGaps: false,
          points: { show: false },
        },
      ],
      hooks: {
        setCursor: [
          (u) => {
            cursor = u.cursor.idx ?? null;
          },
        ],
      },
    };
  }

  // One plot per lane, sharing the cursor; colours are read at each draw, so a theme change redraws.
  $effect(() => {
    const { lanes, x } = view;
    const targets = nodes.slice(0, lanes.length);
    if (targets.length < lanes.length || targets.some((node) => !node)) return;
    const initial = Math.max(1, wrap.clientWidth);
    width = initial;
    plots = lanes.map((lane, i) => {
      const values = lane.log ? lane.values.map((v) => (v === null ? null : Math.max(1, v))) : lane.values;
      return new uPlot(options(lane, initial, x[0] ?? 0, x.at(-1) ?? 1), [x, values], targets[i]);
    });
    const resized = new ResizeObserver(() => {
      const now = Math.max(1, wrap.clientWidth);
      width = now;
      for (const u of plots) u.setSize({ width: now, height: PLOT_H });
    });
    resized.observe(wrap);
    const themed = new MutationObserver(() => {
      for (const u of plots) u.redraw(false);
    });
    themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-scheme"] });
    return () => {
      resized.disconnect();
      themed.disconnect();
      for (const u of plots) u.destroy();
      plots = [];
      cursor = null;
    };
  });

  function readout(i: number): string {
    const s = view.x[i];
    const when = `${formatOffset(s)} · ${formatClock(view.originMs + s * 1000)}`;
    const values = view.lanes.map((lane) => {
      const v = lane.values[i];
      return `${lane.name} ${v === null || v === undefined ? t("ui.occurrence.chart.none") : lane.format(v)}`;
    });
    return `${when} — ${values.join(" · ")}`;
  }

  function show(i: number | null): void {
    for (const u of plots) {
      u.setCursor(i === null ? { left: -10, top: -10 } : { left: u.valToPos(view.x[i], "x"), top: PLOT_H / 2 });
    }
    cursor = i;
  }

  function onkeydown(event: KeyboardEvent): void {
    const n = view.x.length;
    if (!n) return;
    const step = event.shiftKey ? 10 : 1;
    const at = cursor;
    let next: number | null;
    if (event.key === "ArrowRight") next = at === null ? moment : Math.min(n - 1, at + step);
    else if (event.key === "ArrowLeft") next = at === null ? moment : Math.max(0, at - step);
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = n - 1;
    else if (event.key === "Escape") next = null;
    else return;
    event.preventDefault();
    show(next);
  }
</script>

<div class="chart">
  <div
    class="plots"
    bind:this={wrap}
    tabindex="0"
    role="slider"
    aria-label={t("ui.occurrence.chart.label")}
    aria-describedby="{uid}-summary {uid}-keys"
    aria-valuemin={0}
    aria-valuemax={Math.max(0, view.x.length - 1)}
    aria-valuenow={cursor ?? moment}
    aria-valuetext={cursor === null ? t("ui.occurrence.chart.hint") : readout(cursor)}
    {onkeydown}
  >
    {#each view.bands as band, i (i)}
      <span
        class="band {band.tone}"
        style:left="{pct(band.from)}%"
        style:width="{Math.max(0.8, pct(band.to) - pct(band.from))}%"
      ></span>
    {/each}
    {#each view.lanes as lane, i (lane.key)}
      <div class="lane">
        <div class="lanehead" aria-hidden="true">
          <span class="name">{lane.label}</span>
          <span class="peak">{lane.peak}</span>
        </div>
        <div class="plot" bind:this={nodes[i]}></div>
      </div>
    {/each}
    {#each view.markers as marker, i (i)}
      <span class="marker" style:left="{pct(marker.at)}%" title={marker.label}></span>
    {/each}
  </div>
  <div class="axis" aria-hidden="true">
    {#each ticks as s (s)}
      <span style:left="{pct(s)}%" class:start={s === first} class:end={s === last}>{formatOffset(s)}</span>
    {/each}
  </div>
  {#if view.bands.length}
    <div class="bandlabels" aria-hidden="true">
      {#each view.bands as band, i (i)}
        {#if pct(band.from) > 60}
          <span class="flip" style:right="{100 - pct(band.to)}%">{band.label}</span>
        {:else}
          <span style:left="{pct(band.from)}%">{band.label}</span>
        {/if}
      {/each}
    </div>
  {/if}
  <p class="readout" aria-hidden="true">
    {cursor === null ? t("ui.occurrence.chart.hint") : readout(cursor)}
  </p>
  <ul class="sr" id="{uid}-summary">
    {#each view.summary as line, i (i)}
      <li>{line}</li>
    {/each}
  </ul>
  <p class="sr" id="{uid}-keys">{t("ui.occurrence.chart.keys")}</p>
</div>

<style>
  .plots {
    position: relative;
    display: grid;
    gap: 14px;
    padding-bottom: 10px;
    border-radius: 8px;
    outline-offset: 6px;
  }
  .plots:focus-visible {
    outline: 2px solid var(--ink);
  }
  .lane {
    position: relative;
    z-index: 1;
  }
  .lanehead {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 2px;
  }
  .name {
    color: var(--ink2);
    font: 600 12px/1.35 var(--f);
  }
  .peak {
    color: var(--ink3);
    font: 500 11.5px/1.35 var(--f);
    white-space: nowrap;
  }
  .plot {
    height: 52px;
  }
  .plot :global(.uplot) {
    width: 100%;
    font: inherit;
    line-height: 1;
  }
  .plot :global(.u-cursor-x) {
    border-right: 1px solid var(--ink3);
  }
  .band {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 8px;
  }
  .band.freeze {
    background: color-mix(in srgb, var(--freeze) 22%, transparent);
  }
  .band.press {
    background: color-mix(in srgb, var(--press) 12%, transparent);
  }
  .marker {
    position: absolute;
    bottom: 0;
    z-index: 2;
    width: 0;
    height: 0;
    border: 5px solid transparent;
    border-top: 0;
    border-bottom: 8px solid var(--ink2);
    transform: translateX(-50%);
  }
  .axis,
  .bandlabels {
    position: relative;
    height: 18px;
    font: 500 11.5px/18px var(--f);
    color: var(--ink3);
  }
  .axis span,
  .bandlabels span {
    position: absolute;
    top: 0;
    white-space: nowrap;
  }
  .axis span {
    transform: translateX(-50%);
  }
  .axis span.start {
    transform: none;
  }
  .axis span.end {
    transform: translateX(-100%);
  }
  .bandlabels {
    margin-top: 2px;
    color: var(--ink2);
    font-weight: 600;
  }
  .readout {
    margin: 10px 0 0;
    min-height: 1.4em;
    color: var(--ink2);
    font-size: 13px;
  }
  @media (forced-colors: active) {
    .band {
      forced-color-adjust: none;
      background: Highlight;
      opacity: 0.25;
    }
    .marker {
      forced-color-adjust: none;
      border-bottom-color: CanvasText;
    }
  }
</style>
