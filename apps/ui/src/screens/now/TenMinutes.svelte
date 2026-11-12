<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import ChartTip from "../../components/ChartTip.svelte";
  import ToneMark from "../../components/ToneMark.svelte";
  import ZoomBar from "../../components/ZoomBar.svelte";
  import { formatClock, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { LivePoint } from "../../lib/types";
  import { wheelZoom, Zoom } from "../../lib/zoom.svelte";
  import { HEIGHT, RIGHT, SPAN_S, build, laneChart, type LaneChart } from "./chart";
  import { LANES, STATE_TOKEN, laneValue, type LaneKey } from "./lanes";

  let {
    points,
    disk = null,
    paused = false,
    notice,
    onsay,
  }: {
    points: LivePoint[];
    /** The drive letters of the busiest disk now, which name the disk lane. */
    disk?: string | null;
    /** Ctrl is held: the chart stays on the second it was pressed. */
    paused?: boolean;
    /** Set while the monitor is silent: the chart shows the last values and says so. */
    notice?: string;
    onsay: (text: string) => void;
  } = $props();

  const MIN_VIEW_S = 20;

  const laneName = (key: LaneKey) =>
    key === "disk" && disk ? t("ui.now.lane.disk_named", { disk }) : t(`ui.now.lane.${key}`);
  /** Minute spacings for the whole ten minutes; the first that leaves 70 px between labels wins. */
  const WHOLE = [1, 2, 5];
  /** Tick spacings in seconds for a zoomed view; the first that leaves 84 px between labels wins. */
  const STEPS = [5, 10, 15, 30, 60, 120, 300];

  let host: HTMLDivElement;
  let chart: LaneChart | null = null;
  let cursor = $state<{ idx: number; left: number } | null>(null);
  let plot = $state<HTMLElement>();
  let plotWidth = $state(0);
  let outer = $state(0);

  const built = $derived(build(points));
  /** The stretch on screen, in seconds; a zoomed view keeps its place while new seconds arrive. */
  const zoom = new Zoom({
    bounds: () => {
      const to = built.data[0].at(-1) ?? SPAN_S;
      return { from: to - SPAN_S, to };
    },
    width: () => plotWidth,
    minSpan: MIN_VIEW_S,
    inset: { left: 0, right: RIGHT },
  });
  const range = $derived(
    t("ui.zoom.range", { from: formatClock(zoom.from * 1000), to: formatClock((zoom.from + zoom.span) * 1000) }),
  );
  const zoomTicks = $derived.by(() => {
    const view = zoom.view;
    if (!view) return [];
    const width = zoom.plotWidth;
    const every = STEPS.find((step) => (step * width) / zoom.span >= 84) ?? 300;
    const out: { left: number; label: string }[] = [];
    for (let at = Math.ceil(view.from / every) * every; at <= view.to; at += every) {
      const px = ((at - view.from) / zoom.span) * width;
      if (px >= 40 && px <= width - 40) {
        out.push({ left: (px / width) * 100, label: every < 60 ? formatClock(at * 1000) : formatTime(at * 1000) });
      }
    }
    return out;
  });
  /** Minutes before now that the unzoomed axis labels, the left edge left out. */
  const marks = $derived.by(() => {
    const every = WHOLE.find((m) => (m * zoom.plotWidth) / 10 >= 70) ?? 5;
    const out: number[] = [];
    for (let m = 10 - every; m >= 0; m -= every) out.push(m);
    return out;
  });
  const latest = $derived(points.at(-1) ?? null);
  /** Flips with every new second, so the live dot changes as each reading arrives. */
  const beat = $derived(latest !== null && Math.floor(latest.reading.ts_ms / 1000) % 2 === 1);

  /** The second under the readout and how long before the latest one it was. */
  const readout = $derived.by(() => {
    if (!cursor) return null;
    const row = built.rows[cursor.idx];
    const xs = built.data[0];
    if (!row || xs.length === 0) return null;
    return { row, ago: Math.round(xs[xs.length - 1] - xs[cursor.idx]), left: cursor.left };
  });

  const summary = $derived(
    LANES.map((lane) => {
      const values = points.map((p) => lane.value(p.reading)).filter((v) => v !== null);
      const now = latest ? laneValue(lane.value(latest.reading)) : null;
      return now === null || values.length === 0
        ? t("ui.now.chart.summary_none", { lane: laneName(lane.key) })
        : t("ui.now.chart.summary", {
            lane: laneName(lane.key),
            now,
            max: laneValue(Math.max(...values)) ?? "",
          });
    }).join(" "),
  );

  onMount(() => {
    chart = laneChart(host, (idx, left) => {
      cursor = idx === null || left < 0 ? null : { idx, left };
    });
    chart.set(built, zoom.view);
    return () => chart?.destroy();
  });

  // A zoomed view whose seconds have all left the ten minutes gives way to the whole chart.
  $effect(() => {
    if (zoom.view && zoom.view.to <= (built.data[0][0] ?? 0)) zoom.reset();
  });

  $effect(() => {
    chart?.set(built, zoom.view);
  });

  function ago(seconds: number): string {
    if (seconds <= 0) return t("ui.now.ago.now");
    if (seconds < 60) return t("ui.now.ago.seconds", { seconds });
    return t("ui.now.ago.minutes", {
      minutes: Math.floor(seconds / 60),
      seconds: String(seconds % 60).padStart(2, "0"),
    });
  }

  function value(row: LivePoint, lane: (typeof LANES)[number]): string {
    return laneValue(lane.value(row.reading)) ?? t("ui.now.lane.none");
  }

  // Arrows step 5 s (Shift: 60 s), Home and End jump, + − and 0 zoom, Esc takes the readout away.
  function onkeydown(event: KeyboardEvent): void {
    const xs = built.data[0];
    if (!chart || xs.length === 0) return;
    if (zoom.key(event.key, readout ? readout.left : zoom.plotWidth / 2)) {
      event.preventDefault();
      const at = readout?.ago ?? null;
      chart.set(built, zoom.view);
      chart.readAt(at);
      onsay(zoom.view ? t("ui.zoom.said", { range }) : t("ui.zoom.said.all"));
      return;
    }
    const oldest = Math.round(xs[xs.length - 1] - xs[0]);
    const at = readout?.ago ?? null;
    const step = event.shiftKey ? 60 : 5;
    let next: number | null;
    if (event.key === "ArrowLeft") next = Math.min(oldest, (at ?? 0) + step);
    else if (event.key === "ArrowRight") next = Math.max(0, (at ?? 0) - step);
    else if (event.key === "Home") next = oldest;
    else if (event.key === "End") next = 0;
    else if (event.key === "Escape") next = null;
    else return;
    event.preventDefault();
    if (next !== null) {
      zoom.reveal(xs[xs.length - 1] - next);
      chart.set(built, zoom.view);
    }
    chart.readAt(next);
    if (readout) {
      const row = readout.row;
      const values = LANES.map((lane) =>
        t("ui.now.chart.value", { lane: laneName(lane.key), value: value(row, lane) }),
      );
      onsay(t("ui.now.chart.readout", { ago: ago(readout.ago), values: values.join(", ") }));
    }
  }
</script>

{#snippet extra()}
  {#if notice !== undefined}
    <span class="live"><ToneMark tone="silent" />{t("ui.now.tenmin.silent")}</span>
  {:else}
    <span class="live" class:paused>
      <i class:dim={beat && !paused} aria-hidden="true"></i>{t(paused ? "ui.now.tenmin.paused" : "ui.now.tenmin.live")}
    </span>
  {/if}
  <ZoomBar {zoom} {range} {plot} />
{/snippet}

<Block title={t("ui.now.tenmin.title")} actions={extra} {notice}>
<section class="tenmin-card" aria-label={t("ui.now.tenmin.title")} bind:clientWidth={outer}>
  <div class="tenmin" style:grid-template-columns="{outer < 820 ? 120 : 160}px minmax(0, 1fr)">
    <div class="labels">
      {#each LANES as lane (lane.key)}
        <div class="label">
          <span
            class="state-dot"
            style:background="var(--{STATE_TOKEN[latest?.lanes[lane.key] ?? 'suspended']})"
          ></span>
          <span class="name">{laneName(lane.key)}</span>
          <b>{latest ? value(latest, lane) : t("ui.now.lane.none")}</b>
        </div>
      {/each}
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="plot"
      class:pan={zoom.view !== null}
      role="application"
      tabindex="0"
      aria-label={t("ui.now.chart.label")}
      aria-describedby="tenmin-summary"
      bind:this={plot}
      bind:clientWidth={plotWidth}
      use:wheelZoom={zoom}
      onpointerdown={(event) => plot && zoom.press(event, plot)}
      onpointerup={() => zoom.release()}
      onpointermove={(event) => {
        if (zoom.drag(event)) chart?.readAt(null);
      }}
      {onkeydown}
      onfocus={() => {
        if (plot?.matches(":focus-visible") && !readout) chart?.readAt(0);
      }}
      onblur={() => chart?.readAt(null)}
    >
      <div class="canvas" style:height="{HEIGHT}px" bind:this={host}></div>
      {#if readout}
        <ChartTip
          left={readout.left}
          bound={plotWidth}
          heading={ago(readout.ago)}
          rows={LANES.map((lane) => ({
            key: lane.key,
            label: laneName(lane.key),
            value: value(readout.row, lane),
            dot: `var(--${STATE_TOKEN[readout.row.lanes[lane.key]]})`,
          }))}
        />
      {/if}
      <div class="axis" style:margin-right="{RIGHT}px" aria-hidden="true">
        {#if zoom.view}
          {#each zoomTicks as tick (tick.label)}
            <span class="mid" style:left="{tick.left}%">{tick.label}</span>
          {/each}
        {:else}
          {#each marks as minutes (minutes)}
            <span class:mid={minutes > 0} style:left="{100 - minutes * 10}%">
              {minutes === 0 ? t("ui.now.axis.now") : t("ui.now.axis.minutes", { minutes })}
            </span>
          {/each}
        {/if}
      </div>
    </div>
  </div>
  <p class="note">{t("ui.now.tenmin.caption")}</p>
  <p class="sr" id="tenmin-summary">{summary}</p>
</section>
</Block>

<style>
  .live.paused {
    color: var(--ink);
  }
  .live i.dim {
    opacity: 0.4;
  }
  .live.paused i {
    border-radius: 2px;
    background: var(--ink2);
  }
  @media (prefers-reduced-motion: reduce) {
    .live i.dim {
      opacity: 1;
    }
  }
  .tenmin {
    display: grid;
    column-gap: 12px;
    align-items: start;
  }
  .labels {
    display: grid;
    grid-auto-rows: 58px;
    align-content: start;
  }
  .label {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    border-top: 1px solid var(--stroke2);
  }
  .label:first-child {
    border-top: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    color: var(--ink2);
    font: 600 12.5px var(--f);
  }
  .label b {
    color: var(--ink);
    font: 600 16px var(--f);
  }
  .plot {
    position: relative;
    min-width: 0;
    border-radius: 8px;
  }
  /* Keyboard focus shows as the readout at the newest second, not as a ring. */
  .plot:focus-visible {
    outline: none;
  }
  .canvas :global(.u-cursor-x) {
    border-right: 1px solid var(--ink3);
  }
  .axis {
    position: relative;
    height: 22px;
    color: var(--ink3);
    font: 500 11.5px/22px var(--f);
  }
  .axis span {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    white-space: nowrap;
  }
  .axis span {
    transform: translateX(-100%);
  }
  .axis span.mid {
    transform: translateX(-50%);
  }
  .plot.pan {
    cursor: grab;
  }
  .plot.pan:active {
    cursor: grabbing;
  }
  @media (forced-colors: active) {
    .live i {
      forced-color-adjust: none;
    }
  }
</style>
