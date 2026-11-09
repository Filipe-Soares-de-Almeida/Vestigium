<script lang="ts">
  import { untrack } from "svelte";
  import uPlot from "uplot";
  import "uplot/dist/uPlot.min.css";

  import type { Snippet } from "svelte";

  import Block from "../../components/Block.svelte";
  import ChartTip from "../../components/ChartTip.svelte";
  import ZoomBar from "../../components/ZoomBar.svelte";
  import { formatClock, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { wheelZoom, Zoom } from "../../lib/zoom.svelte";
  import type { ChartView, Lane } from "./chart";
  import { formatOffset } from "./format";

  let {
    view,
    title,
    children,
    clock = false,
    label = t("ui.occurrence.chart.label"),
    hint = t("ui.occurrence.chart.hint"),
    keys = t("ui.occurrence.chart.keys"),
  }: {
    view: ChartView;
    /** The block's title; its zoom buttons sit beside it. */
    title: string;
    /** Notes under the chart. */
    children?: Snippet;
    /** Labels the axis with the time of day instead of seconds from the origin. */
    clock?: boolean;
    label?: string;
    hint?: string;
    keys?: string;
  } = $props();

  /** A lane's height; its name and peak sit in the top 24 px. */
  const PLOT_H = 84;
  /** Axis steps in seconds, and the room one label needs before the next step is used. */
  const TICK_STEPS = [5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600];
  const TICK_ROOM_PX = 56;
  const MIN_VIEW_S = 10;
  /** The space one event dot needs before the next gets its own. */
  const MARK_ROOM_PX = 10;
  const uid = `occ-chart-${Math.random().toString(36).slice(2, 8)}`;

  let wrap = $state<HTMLDivElement>();
  let nodes = $state<HTMLDivElement[]>([]);
  /** The point the readout shows, from the mouse or the keyboard; the slider speaks it. */
  let cursor = $state<number | null>(null);
  /** Pixels from the left of the plots to the point under the readout. */
  let cursorLeft = $state(0);
  let plots = $state.raw<uPlot[]>([]);
  let width = $state(0);

  const first = $derived(view.x[0] ?? 0);
  const last = $derived(view.x.at(-1) ?? 0);
  const zoom = new Zoom({
    bounds: () => ({ from: first, to: last }),
    width: () => width,
    minSpan: MIN_VIEW_S,
  });
  /** The stretch on screen, in seconds from the start of the occurrence. */
  const lo = $derived(zoom.view?.from ?? first);
  const hi = $derived(zoom.view?.to ?? last);
  /** The first point at or after the start of the occurrence, where the keyboard starts. */
  const moment = $derived(Math.max(0, view.x.findIndex((s) => s >= 0)));
  const pct = (s: number) => ((s - lo) / Math.max(1, hi - lo)) * 100;
  /** A stretch's place on the axis, cut to what is on screen; `null` when none of it is. */
  const place = (from: number, to: number) => {
    const left = Math.max(0, pct(from));
    const right = Math.min(100, pct(to));
    return right <= 0 || left >= 100 ? null : { left, width: Math.max(0.8, right - left) };
  };
  // Time-of-day ticks fall on whole minutes and seconds of the clock and need room for the seconds.
  const ticks = $derived.by(() => {
    const perSecond = width / Math.max(1, hi - lo);
    const room = clock ? TICK_ROOM_PX + 20 : TICK_ROOM_PX;
    const step = TICK_STEPS.find((s) => s * perSecond >= room) ?? TICK_STEPS.at(-1)!;
    const shift = clock ? (view.originMs / 1000) % step : 0;
    const out: number[] = [];
    for (let s = Math.ceil((lo + shift) / step) * step - shift; s <= hi; s += step) out.push(s);
    return out;
  });
  const range = $derived(t("ui.zoom.range", { from: tickLabel(lo), to: tickLabel(hi) }));
  /** Events on screen; two closer than one dot's room share a dot, so they never pile up. */
  const marks = $derived.by(() => {
    const out: { at: number; label: string }[] = [];
    const room = (MARK_ROOM_PX / Math.max(1, width)) * 100;
    for (const m of [...view.markers].sort((a, b) => a.at - b.at)) {
      const at = pct(m.at);
      if (at < 0 || at > 100) continue;
      const near = out.at(-1);
      if (near && at - near.at < room) near.label += `
${m.label}`;
      else out.push({ at, label: m.label });
    }
    return out;
  });

  /** A place on the axis: seconds from the origin, or the time of day, to the second while steps are short. */
  function tickLabel(s: number): string {
    if (!clock) return formatOffset(s);
    const ms = view.originMs + s * 1000;
    return (ticks[1] ?? Infinity) - (ticks[0] ?? 0) < 60 ? formatClock(ms) : formatTime(ms);
  }

  function token(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  function options(lane: Lane, width: number, x0: number, x1: number): uPlot.Options {
    const peak = Math.max(1, ...lane.values.map((v) => v ?? 0));
    return {
      width,
      height: PLOT_H,
      padding: [24, 0, 8, 0],
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
          spanGaps: false,
          points: { show: false },
        },
      ],
      hooks: {
        setCursor: [
          (u) => {
            cursor = u.cursor.idx ?? null;
            cursorLeft = u.cursor.left ?? 0;
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
    const host = wrap;
    if (!host) return;
    const initial = Math.max(1, host.clientWidth);
    width = initial;
    const made = lanes.map((lane, i) => {
      const values = lane.log ? lane.values.map((v) => (v === null ? null : Math.max(1, v))) : lane.values;
      return new uPlot(options(lane, initial, x[0] ?? 0, x.at(-1) ?? 1), [x, values], targets[i]);
    });
    plots = made;
    const resized = new ResizeObserver(() => {
      const now = Math.max(1, host.clientWidth);
      width = now;
      for (const u of made) u.setSize({ width: now, height: PLOT_H });
    });
    resized.observe(host);
    const themed = new MutationObserver(() => {
      for (const u of made) u.redraw(false);
    });
    themed.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "data-scheme"] });
    return () => {
      resized.disconnect();
      themed.disconnect();
      for (const u of made) u.destroy();
      plots = [];
      cursor = null;
    };
  });

  // Every lane shows the zoomed stretch; the readout moves with the point it reads.
  $effect(() => {
    const [min, max] = [lo, hi];
    for (const u of plots) u.setScale("x", { min, max });
    // uPlot applies a new scale in a microtask; read the position after it.
    queueMicrotask(() =>
      untrack(() => {
        if (cursor !== null) show(cursor);
      }),
    );
  });

  function readout(i: number): string {
    const values = view.lanes.map((lane) => `${lane.name} ${value(lane, i)}`);
    const extra = context(i).map((row) => row.label);
    return `${heading(i)} — ${[...extra, ...values].join(" · ")}`;
  }

  function show(i: number | null): void {
    for (const u of plots) {
      u.setCursor(i === null ? { left: -10, top: -10 } : { left: u.valToPos(view.x[i], "x"), top: PLOT_H / 2 });
    }
    cursor = i;
    if (i !== null && plots[0]) cursorLeft = plots[0].valToPos(view.x[i], "x");
  }

  function heading(i: number): string {
    const s = view.x[i];
    const time = formatClock(view.originMs + s * 1000);
    return clock ? time : `${formatOffset(s)} · ${time}`;
  }

  /** What the readout adds at a point: the span it is in, then the events of that second. */
  function context(i: number): { key: string; label: string; value: string }[] {
    const s = view.x[i];
    const spans = view.bands.filter((b) => s >= b.from - 0.5 && s <= b.to + 0.5).map((b) => b.label);
    const events = view.markers.filter((m) => Math.abs(m.at - s) < 0.5).map((m) => m.label);
    return [...spans, ...events].map((label, n) => ({ key: `context-${n}`, label, value: "" }));
  }

  function value(lane: Lane, i: number): string {
    const v = lane.values[i];
    return v === null || v === undefined ? t("ui.occurrence.chart.none") : lane.format(v);
  }

  // Arrows step a point (Shift: ten), Home and End jump, + − and 0 zoom, Esc takes the readout away.
  function onkeydown(event: KeyboardEvent): void {
    const n = view.x.length;
    if (!n) return;
    if (zoom.key(event.key, cursor === null ? width / 2 : cursorLeft)) {
      event.preventDefault();
      return;
    }
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
    if (next !== null) zoom.reveal(view.x[next]);
    show(next);
  }
</script>

{#snippet bar()}
  <ZoomBar {zoom} {range} plot={wrap} />
{/snippet}

<Block {title} actions={bar}>
<div class="chart">
  {#if view.bands.length}
    <div class="bandlabels" aria-hidden="true">
      {#each view.bands as band, i (i)}
        {@const at = place(band.from, band.to)}
        {#if at && at.left > 60}
          <span class="flip" style:right="{100 - at.left - at.width}%">{band.label}</span>
        {:else if at}
          <span style:left="{at.left}%">{band.label}</span>
        {/if}
      {/each}
    </div>
  {/if}
  <div
    class="plots"
    class:pan={zoom.view !== null}
    bind:this={wrap}
    use:wheelZoom={zoom}
    onpointerdown={(event) => wrap && zoom.press(event, wrap)}
    onpointerup={() => zoom.release()}
    onpointermove={(event) => {
      if (zoom.drag(event)) show(null);
    }}
    onfocus={() => {
      if (wrap?.matches(":focus-visible") && cursor === null) show(moment);
    }}
    onblur={() => show(null)}
    tabindex="0"
    role="slider"
    aria-label={label}
    aria-describedby="{uid}-summary {uid}-keys"
    aria-valuemin={0}
    aria-valuemax={Math.max(0, view.x.length - 1)}
    aria-valuenow={cursor ?? moment}
    aria-valuetext={cursor === null ? hint : readout(cursor)}
    {onkeydown}
  >
    {#each view.bands as band, i (i)}
      {@const at = place(band.from, band.to)}
      {#if at}
        <span class="band {band.tone}" class:blank={band.blank} style:left="{at.left}%" style:width="{at.width}%"></span>
      {/if}
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
    {#each marks as mark (mark.at)}
      <span class="marker" style:left="{mark.at}%" title={mark.label}></span>
    {/each}
    {#if cursor !== null}
      {@const at = cursor}
      <ChartTip
        left={cursorLeft}
        bound={width}
        heading={heading(at)}
        top={22}
        rows={[
          ...context(at),
          ...view.lanes.map((lane) => ({
            key: lane.key,
            label: lane.name,
            value: value(lane, at),
            dot: `var(--${lane.tone})`,
          })),
        ]}
      />
    {/if}
  </div>
  <div class="axis" aria-hidden="true">
    {#each ticks as s (s)}
      <span style:left="{pct(s)}%" class:start={pct(s) < 4} class:end={pct(s) > 96}>{tickLabel(s)}</span>
    {/each}
  </div>
  <ul class="sr" id="{uid}-summary">
    {#each view.summary as line, i (i)}
      <li>{line}</li>
    {/each}
  </ul>
  <p class="sr" id="{uid}-keys">{keys}</p>
</div>
{@render children?.()}
</Block>

<style>
  .plots {
    position: relative;
    display: grid;
    border-radius: 4px;
  }
  /* Keyboard focus shows as the readout at the moment, not as a ring. */
  .plots:focus-visible {
    outline: none;
  }
  .plots.pan {
    cursor: grab;
  }
  .plots.pan:active {
    cursor: grabbing;
  }
  .lane {
    position: relative;
    z-index: 1;
  }
  .lane + .lane {
    border-top: 1px solid var(--stroke2);
  }
  .lanehead {
    position: absolute;
    top: 5px;
    right: 4px;
    left: 4px;
    z-index: 1;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    pointer-events: none;
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
    height: 84px;
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
    top: -4px;
    bottom: 0;
    border-radius: 4px;
  }
  .band.freeze {
    background: var(--freeze-t);
  }
  /* Nothing was read here, so the lines break: a light stripe instead of a solid block. */
  .band.freeze.blank {
    border-radius: 2px;
    background:
      repeating-linear-gradient(135deg, color-mix(in srgb, var(--freeze) 26%, transparent) 0 1.5px, transparent 1.5px 6px),
      color-mix(in srgb, var(--freeze) 6%, transparent);
    box-shadow:
      inset 1px 0 0 color-mix(in srgb, var(--freeze) 55%, transparent),
      inset -1px 0 0 color-mix(in srgb, var(--freeze) 55%, transparent);
  }
  .band.press {
    background: color-mix(in srgb, var(--press) 12%, transparent);
  }
  /* An event of the moment: a dashed line through the lanes with a dot at the top. */
  .marker {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 2;
    width: 0;
    border-left: 1px dashed var(--freeze);
  }
  .marker::before {
    content: "";
    position: absolute;
    top: -1.5px;
    left: -4px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--freeze);
  }
  .axis,
  .bandlabels {
    position: relative;
    height: 24px;
    font: 500 11.5px/24px var(--f);
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
    height: 18px;
    color: var(--ink2);
    font: 600 12px/18px var(--f);
  }
  .bandlabels span {
    padding-left: 6px;
  }
  .bandlabels span.flip {
    padding: 0 6px 0 0;
  }
  @media (forced-colors: active) {
    .band {
      forced-color-adjust: none;
      background: Highlight;
      opacity: 0.25;
    }
    .marker {
      forced-color-adjust: none;
      border-left-color: CanvasText;
    }
    .marker::before {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
