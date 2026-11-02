<script lang="ts">
  import ZoomBar from "../../components/ZoomBar.svelte";
  import { formatClock, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { CaptureDto, DetailLevel } from "../../lib/types";
  import { wheelZoom, Zoom } from "../../lib/zoom.svelte";
  import { alignment, type Mark } from "./model";
  import { markLine } from "./words";

  let { dto, level, summary }: { dto: CaptureDto; level: DetailLevel; summary: string } = $props();

  const TOP = 26;
  const LANE = 38;
  const HEIGHT = TOP + 3 * LANE + 24;
  const LANES = [0, 1, 2] as const;
  const PAD = 8;
  const MIN_VIEW_MS = 10_000;
  /** Tick spacings for a zoomed view; the first that leaves 72 px between labels wins. */
  const STEPS = [5, 10, 15, 30, 60, 120, 300, 600].map((seconds) => seconds * 1000);

  const al = $derived(alignment(dto));
  const total = $derived(Math.max(1, dto.end_ms - dto.start_ms));
  const minutes = $derived(Math.max(1, Math.round(total / 60_000)));

  let width = $state(600);
  let plot = $state<HTMLElement>();
  /** The mark the readout shows, from the pointer or the keyboard. */
  let cursor = $state(-1);
  let said = $state("");
  let tipWidth = $state(0);
  let focused = false;

  /** The stretch of the capture on screen, in milliseconds from its start. */
  const zoom = new Zoom({
    bounds: () => ({ from: 0, to: total }),
    width: () => width,
    minSpan: MIN_VIEW_MS,
    inset: { left: PAD, right: PAD },
  });
  const vFrom = $derived(zoom.from);
  const vSpan = $derived(zoom.span);

  const x = (at: number) => PAD + ((at - vFrom) / vSpan) * (width - 2 * PAD);
  /** Whether `at` falls within the stretch on screen, give or take a mark's width. */
  const shown = (at: number) => x(at) > -16 && x(at) < width + 16;
  const y = (lane: number) => TOP + lane * LANE + LANE / 2;
  const radius = (m: Mark) =>
    Math.min(9, m.lane === 0 ? 3.5 + m.value / 9 : m.lane === 1 ? 3 + m.value / 260 : 2.8 + m.value / 34);
  type Tick = { at: number; label: string; anchor: "start" | "middle" | "end" };
  const ticks = $derived.by((): Tick[] => {
    if (zoom.view) {
      const every = STEPS.find((step) => (step * (width - 2 * PAD)) / vSpan >= 72) ?? 600_000;
      const clock = every < 60_000 ? formatClock : formatTime;
      const out: Tick[] = [];
      for (let ts = Math.ceil((dto.start_ms + vFrom) / every) * every; ts <= dto.start_ms + vFrom + vSpan; ts += every) {
        const px = x(ts - dto.start_ms);
        if (px >= 36 && px <= width - 36) out.push({ at: ts - dto.start_ms, label: clock(ts), anchor: "middle" });
      }
      return out;
    }
    const step = minutes >= 10 ? 2 : 1;
    return Array.from({ length: Math.floor(minutes / step) + 1 }, (_, i): Tick => {
      const at = i * step * 60_000;
      return { at, label: formatTime(dto.start_ms + at), anchor: i === 0 ? "start" : i * step === minutes ? "end" : "middle" };
    });
  });
  const range = $derived(
    t("ui.zoom.range", { from: formatClock(dto.start_ms + vFrom), to: formatClock(dto.start_ms + vFrom + vSpan) }),
  );
  const current = $derived(cursor >= 0 ? (al.marks[cursor] ?? null) : null);
  const group = $derived(current ? (current.moment >= 0 ? al.marks.filter((m) => m.moment === current.moment) : [current]) : []);
  const tipLeft = $derived(current ? Math.max(0, Math.min(width - tipWidth, x(current.at) - tipWidth / 2)) : 0);

  // Another capture starts with the whole range on screen.
  $effect(() => {
    void dto.start_ms;
    zoom.reset();
  });

  function line(m: Mark): string {
    const median = m.lane === 2 ? (dto.hitches.find((h) => h.ts_ms === m.ts_ms && h.app === m.who)?.median_ms ?? null) : null;
    return markLine(m.lane, m.value, m.who, level, median);
  }

  function say(): void {
    const m = al.marks[cursor];
    if (!m) return;
    said = `${formatClock(m.ts_ms)}: ${group.map(line).join("; ")}.`;
  }

  function set(next: number): void {
    cursor = next;
    if (next < 0) return;
    zoom.reveal(al.marks[next].at);
    say();
  }

  function onpointermove(event: PointerEvent): void {
    if (zoom.drag(event)) {
      cursor = -1;
      return;
    }
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const px = event.clientX - box.left;
    const lane = Math.floor((event.clientY - box.top - TOP) / LANE);
    let best = -1;
    let nearest = 14;
    al.marks.forEach((m, i) => {
      const d = Math.abs(x(m.at) - px);
      if ((m.lane === lane || (m.moment >= 0 && d < 11)) && d < nearest) {
        nearest = d;
        best = i;
      }
    });
    if (best !== cursor) cursor = best;
  }

  function onfocus(): void {
    focused = true;
    if (cursor < 0) set(Math.max(0, al.marks.findIndex((m) => m.moment === 0)));
  }

  // Arrows step mark by mark, up and down jump between moments, Home and End jump, Esc clears.
  function onkeydown(event: KeyboardEvent): void {
    const n = al.marks.length;
    if (!n) return;
    if (zoom.key(event.key, cursor >= 0 ? x(al.marks[cursor].at) : width / 2)) {
      event.preventDefault();
      said = zoom.view ? t("ui.zoom.said", { range }) : t("ui.zoom.said.all");
      return;
    }
    let next = -2;
    if (event.key === "ArrowRight") next = Math.min(n - 1, cursor + 1);
    else if (event.key === "ArrowLeft") next = Math.max(0, cursor - 1);
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = n - 1;
    else if (event.key === "Escape") next = -1;
    else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      const at = cursor >= 0 ? al.marks[cursor].at : -1;
      const order = al.moments.map((m, i) => [m, i] as const);
      const found = event.key === "ArrowDown" ? order.find(([m]) => m > at + 1) : order.reverse().find(([m]) => m < at - 1);
      if (found) next = al.marks.findIndex((m) => m.moment === found[1]);
    }
    if (next === -2) return;
    event.preventDefault();
    set(next);
  }
</script>

<div class="align">
  <div class="labels" aria-hidden="true">
    {#each LANES as lane (lane)}
      <div>{t(level === "Simple" ? `ui.captures.lane.${lane}` : `ui.captures.lane.${lane}.d`)}</div>
    {/each}
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="plot"
    role="application"
    tabindex="0"
    aria-label={t("ui.captures.chart.label")}
    aria-describedby="align-summary"
    bind:this={plot}
    bind:clientWidth={width}
    use:wheelZoom={zoom}
    class:pan={zoom.view !== null}
    onpointerdown={(event) => plot && zoom.press(event, plot)}
    onpointerup={() => zoom.release()}
    {onpointermove}
    onpointerleave={() => {
      if (!focused) cursor = -1;
    }}
    {onfocus}
    onblur={() => {
      focused = false;
      cursor = -1;
    }}
    {onkeydown}
  >
    <svg viewBox="0 0 {width} {HEIGHT}" height={HEIGHT} aria-hidden="true">
      {#each LANES as lane (lane)}
        <line x1="0" x2={width} y1={y(lane)} y2={y(lane)} class="guide" />
      {/each}
      {#each al.moments as at, i (i)}
        {#if shown(at)}
          <rect class="band" x={x(at) - 11} y={TOP - 2} width="22" height={3 * LANE + 2} rx="11" />
          <text class="num" x={x(at)} y={TOP - 9} text-anchor="middle">{i + 1}</text>
        {/if}
      {/each}
      {#if dto.frames !== "Ok"}
        <text class="axis" x={width / 2} y={y(2) + 4} text-anchor="middle">{t("ui.captures.chart.noframes")}</text>
      {/if}
      {#each al.marks as m, i (i)}
        {#if shown(m.at)}
          <circle class="mark" class:moment={m.moment >= 0} cx={x(m.at)} cy={y(m.lane)} r={radius(m)} />
          {#if i === cursor}
            <circle class="ring" cx={x(m.at)} cy={y(m.lane)} r={radius(m) + 3.5} />
          {/if}
        {/if}
      {/each}
      {#each ticks as tick (tick.at)}
        <text class="axis" x={x(tick.at)} y={HEIGHT - 4} text-anchor={tick.anchor}>{tick.label}</text>
      {/each}
    </svg>
    {#if current}
      <div class="tip" style:left="{tipLeft}px" style:bottom="{HEIGHT - TOP + 10}px" bind:clientWidth={tipWidth}>
        <b>{current.moment >= 0 ? t("ui.captures.chart.moment", { n: current.moment + 1 }) : t("ui.captures.chart.alone")}</b>
        <span class="time">{formatClock(current.ts_ms)}</span>
        <ul>
          {#each group as m, i (i)}
            <li>
              <span class="state-dot" style:background="var(--{m.moment >= 0 ? 'freeze' : 'busy'})"></span>
              <span>{#if m === current}<b>{line(m)}</b>{:else}{line(m)}{/if}</span>
            </li>
          {/each}
        </ul>
        {#if current.moment < 0}<p class="hint">{t("ui.captures.chart.alone.hint")}</p>{/if}
      </div>
    {/if}
  </div>
</div>
<div class="key" aria-hidden="true">
  <span><span class="state-dot" style:background="var(--freeze)"></span>{t("ui.captures.chart.key.moment")}</span>
  <span><span class="state-dot" style:background="var(--busy)"></span>{t("ui.captures.chart.key.alone")}</span>
  <span>{t("ui.captures.chart.key.size")}</span>
</div>
<ZoomBar {zoom} {range} {plot} />
<p class="summary" id="align-summary">{summary}</p>
<p class="sr" aria-live="polite">{said}</p>

<style>
  .align {
    display: grid;
    grid-template-columns: 172px minmax(0, 1fr);
    gap: 12px;
    margin-top: 4px;
  }
  .labels {
    display: grid;
    grid-template-rows: repeat(3, 38px);
    align-content: start;
    padding-top: 26px;
  }
  .labels div {
    display: flex;
    align-items: center;
    color: var(--ink2);
    font: 600 12.5px/1.25 var(--f);
  }
  .plot {
    position: relative;
    min-width: 0;
    outline: none;
  }
  .plot:focus-visible {
    outline: none;
  }
  .plot.pan {
    cursor: grab;
  }
  .plot.pan:active {
    cursor: grabbing;
  }
  svg {
    display: block;
    width: 100%;
    overflow: visible;
  }
  .plot.pan svg {
    overflow: hidden;
  }
  .guide {
    stroke: var(--stroke);
    stroke-dasharray: 1 5;
    stroke-linecap: round;
  }
  .band {
    fill: var(--freeze);
    fill-opacity: 0.1;
  }
  .num {
    fill: var(--ink2);
    font: 600 11.5px var(--f);
  }
  .axis {
    fill: var(--ink3);
    font: 500 11.5px var(--f);
  }
  .mark {
    fill: var(--busy);
    stroke: var(--card);
    stroke-width: 2;
  }
  .mark.moment {
    fill: var(--freeze);
  }
  .ring {
    fill: none;
    stroke: var(--ink);
    stroke-width: 2;
  }
  .tip {
    position: absolute;
    z-index: 3;
    width: 300px;
    max-width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--card);
    box-shadow: var(--shadow-float);
    font-size: 13px;
    pointer-events: none;
  }
  .tip > b {
    font: 600 15px/1.3 var(--f);
  }
  .time {
    margin-left: 8px;
    color: var(--ink2);
  }
  .tip ul {
    display: grid;
    gap: 4px;
    margin: 6px 0 0;
    padding: 0;
    color: var(--ink2);
    list-style: none;
  }
  .tip li {
    display: grid;
    grid-template-columns: 12px minmax(0, 1fr);
    gap: 6px;
    align-items: baseline;
  }
  .tip li b {
    color: var(--ink);
    font-weight: 600;
  }
  .state-dot {
    width: 7px;
    height: 7px;
  }
  .hint {
    margin: 6px 0 0;
    color: var(--ink3);
  }
  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin-top: 10px;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .key .state-dot {
    width: 8px;
    height: 8px;
    margin-right: 6px;
  }
  .summary {
    margin: 10px 0 0;
    max-width: 80ch;
    font-size: 13.5px;
    line-height: 1.5;
  }
  @container content (max-width: 720px) {
    .align {
      grid-template-columns: 120px minmax(0, 1fr);
    }
  }
  @media (forced-colors: active) {
    .mark,
    .band,
    .ring {
      forced-color-adjust: none;
    }
    .mark {
      fill: CanvasText;
      stroke: Canvas;
    }
    .mark.moment {
      fill: Highlight;
    }
    .band {
      fill: Highlight;
    }
    .ring {
      stroke: CanvasText;
    }
    .tip {
      border-color: CanvasText;
    }
  }
</style>
