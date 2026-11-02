<script lang="ts">
  import { onMount } from "svelte";

  import { t } from "../../lib/i18n.svelte";
  import type { LivePoint } from "../../lib/types";
  import { HEIGHT, RIGHT, build, laneChart, type LaneChart } from "./chart";
  import { LANES, STATE_TOKEN, laneValue } from "./lanes";

  let { points, onsay }: { points: LivePoint[]; onsay: (text: string) => void } = $props();

  const AXIS = [10, 8, 6, 4, 2, 0];
  /** Below this plot width only the two ends of the axis are labelled. */
  const AXIS_FULL_FROM = 300;

  let host: HTMLDivElement;
  let chart: LaneChart | null = null;
  let cursor = $state<{ idx: number; left: number } | null>(null);
  let plotWidth = $state(0);
  let tipWidth = $state(0);

  const built = $derived(build(points));
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

  const tipLeft = $derived(
    readout
      ? readout.left + 14 + tipWidth > plotWidth
        ? readout.left - 14 - tipWidth
        : readout.left + 14
      : 0,
  );

  const summary = $derived(
    LANES.map((lane) => {
      const values = points.map((p) => lane.value(p.reading)).filter((v) => v !== null);
      const now = latest ? laneValue(lane.value(latest.reading)) : null;
      return now === null || values.length === 0
        ? t("ui.now.chart.summary_none", { lane: t(`ui.now.lane.${lane.key}`) })
        : t("ui.now.chart.summary", {
            lane: t(`ui.now.lane.${lane.key}`),
            now,
            max: laneValue(Math.max(...values)) ?? "",
          });
    }).join(" "),
  );

  onMount(() => {
    chart = laneChart(host, (idx, left) => {
      cursor = idx === null || left < 0 ? null : { idx, left };
    });
    chart.set(built);
    return () => chart?.destroy();
  });

  $effect(() => {
    chart?.set(built);
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

  // Arrows step 5 s (Shift: 60 s), Home and End jump, Esc takes the readout away.
  function onkeydown(event: KeyboardEvent): void {
    const xs = built.data[0];
    if (!chart || xs.length === 0) return;
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
    chart.readAt(next);
    if (readout) {
      const row = readout.row;
      const values = LANES.map((lane) =>
        t("ui.now.chart.value", { lane: t(`ui.now.lane.${lane.key}`), value: value(row, lane) }),
      );
      onsay(t("ui.now.chart.readout", { ago: ago(readout.ago), values: values.join(", ") }));
    }
  }
</script>

<section class="card tenmin-card" aria-labelledby="tenmin-title">
  <div class="head">
    <h2 id="tenmin-title">{t("ui.now.tenmin.title")}</h2>
    <span class="live"><i class:dim={beat}></i>{t("ui.now.tenmin.live")}</span>
  </div>
  <p class="caption">{t("ui.now.tenmin.caption")}</p>
  <div class="tenmin">
    <div class="labels">
      {#each LANES as lane (lane.key)}
        <div class="label">
          <span
            class="state-dot"
            style:background="var(--{STATE_TOKEN[latest?.lanes[lane.key] ?? 'suspended']})"
          ></span>
          <span>
            {t(`ui.now.lane.${lane.key}`)}
            <b>{latest ? value(latest, lane) : t("ui.now.lane.none")}</b>
          </span>
        </div>
      {/each}
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="plot"
      role="application"
      tabindex="0"
      aria-label={t("ui.now.chart.label")}
      aria-describedby="tenmin-summary"
      bind:clientWidth={plotWidth}
      {onkeydown}
      onblur={() => chart?.readAt(null)}
    >
      <div class="canvas" style:height="{HEIGHT}px" bind:this={host}></div>
      {#if readout}
        <div class="tip" style:left="{tipLeft}px" bind:clientWidth={tipWidth}>
          <b>{ago(readout.ago)}</b>
          <div class="rows">
            {#each LANES as lane (lane.key)}
              <span>
                <span
                  class="state-dot"
                  style:background="var(--{STATE_TOKEN[readout.row.lanes[lane.key]]})"
                ></span>
                {t(`ui.now.lane.${lane.key}`)}
                <b>{value(readout.row, lane)}</b>
              </span>
            {/each}
          </div>
        </div>
      {/if}
      <div class="axis" style:margin-right="{RIGHT}px" aria-hidden="true">
        {#each AXIS as minutes (minutes)}
          {#if minutes === 10 || minutes === 0 || plotWidth >= AXIS_FULL_FROM}
            <span style:left="{100 - minutes * 10}%">
              {minutes === 0 ? t("ui.now.axis.now") : t("ui.now.axis.minutes", { minutes })}
            </span>
          {/if}
        {/each}
      </div>
    </div>
  </div>
  <p class="sr" id="tenmin-summary">{summary}</p>
</section>

<style>
  .tenmin-card {
    container: tenmin / inline-size;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 4px 12px;
    margin-bottom: 10px;
  }
  .head h2 {
    margin: 0;
  }
  .live {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--ink2);
    font-size: 12.5px;
    white-space: nowrap;
  }
  .live i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
  }
  .live i.dim {
    opacity: 0.45;
  }
  @media (prefers-reduced-motion: reduce) {
    .live i.dim {
      opacity: 1;
    }
  }
  .caption {
    margin: -4px 0 12px;
    color: var(--ink2);
    font-size: 13px;
  }
  .tenmin {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    gap: 12px;
    margin-top: 6px;
  }
  .labels {
    display: grid;
    grid-auto-rows: 56px;
    align-content: start;
  }
  .label {
    display: flex;
    gap: 8px;
    align-items: center;
    border-top: 1px solid var(--stroke2);
    color: var(--ink2);
    font: 600 12.5px/1.25 var(--f);
    min-width: 0;
  }
  .label:first-child {
    border-top: 0;
  }
  .label b {
    display: block;
    color: var(--ink);
    font: 600 16px/1.25 var(--f);
  }
  .plot {
    position: relative;
    min-width: 0;
    border-radius: 8px;
  }
  .plot:focus-visible {
    outline-offset: 6px;
  }
  .canvas :global(.u-cursor-x) {
    border-right: 1px solid var(--ink3);
  }
  .tip {
    position: absolute;
    top: 4px;
    z-index: 3;
    min-width: 210px;
    max-width: 340px;
    padding: 10px 12px;
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: 12px;
    box-shadow: var(--shadow-float);
    font-size: 13px;
    pointer-events: none;
  }
  .tip > b {
    font: 600 15px/1.3 var(--f);
  }
  .rows {
    display: grid;
    gap: 4px;
    margin-top: 6px;
    color: var(--ink2);
  }
  .rows b {
    margin-left: 2px;
    color: var(--ink);
    font-weight: 600;
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
  .axis span:first-child {
    transform: none;
  }
  .axis span:last-child {
    transform: translateX(-100%);
  }
  @container content (max-width: 720px) {
    .tenmin {
      grid-template-columns: 118px minmax(0, 1fr);
    }
  }
  @container tenmin (max-width: 420px) {
    .tenmin {
      grid-template-columns: 92px minmax(0, 1fr);
      gap: 8px;
    }
    .label {
      gap: 6px;
    }
  }
  @media (forced-colors: active) {
    .live i {
      forced-color-adjust: none;
    }
    .tip {
      border-color: CanvasText;
    }
  }
</style>
