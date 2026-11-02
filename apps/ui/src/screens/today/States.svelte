<script lang="ts">
  import { formatDuration, formatPercent, formatTime } from "../../lib/format";
  import { lang, t, tn } from "../../lib/i18n.svelte";
  import { go, href } from "../../lib/router.svelte";
  import type { DetailLevel, OccurrenceSummary, PcState } from "../../lib/types";
  import { type Day, gapAt, HOUR_MS, pointAt, type Run, STATE_TOKEN, STATES, toneOf } from "./day";
  import { causeName, kindName, occurrenceTitle } from "./text";

  let { day, level }: { day: Day; level: DetailLevel } = $props();

  // Rows are 28 px with a 22 px band above them for the marks and 26 px below for the axis.
  const ROW = 28;
  const TOP = 22;
  const HEIGHT = TOP + STATES.length * ROW + 26;
  const MIN_SPAN_MS = 6 * HOUR_MS;
  const MINUTE_MS = 60_000;
  const MARK_NEAR_MS = 3 * MINUTE_MS;
  const MIN_VIEW_MS = 20 * MINUTE_MS;
  /** Tick spacings for a zoomed view; the first that leaves 52 px between labels wins. */
  const STEPS = [5, 10, 15, 30, 60, 120, 180, 360].map((minutes) => minutes * MINUTE_MS);

  type Detail = { mark: OccurrenceSummary } | { run: Run; at: number };

  let width = $state(0);
  let filter = $state<PcState | null>(null);
  let cursor = $state<number | null>(null);
  let near = $state.raw<OccurrenceSummary | null>(null);
  let hovered = $state.raw<Run | null>(null);
  let card = $state<HTMLElement>();
  let plot = $state<HTMLElement>();
  let tip = $state<HTMLElement>();
  /** The stretch of the day on screen; null shows the whole day. */
  let view = $state<{ from: number; to: number } | null>(null);
  let drag: { x: number; from: number; moved: boolean } | null = null;
  let dragged = false;

  const span = $derived(Math.max(day.now - day.start, MIN_SPAN_MS));
  const w = $derived(width || 700);
  const vFrom = $derived(view?.from ?? day.start);
  const vSpan = $derived(view ? view.to - view.from : span);
  const x = (ms: number) => ((ms - vFrom) / vSpan) * w;
  const rowOf = (state: PcState) => STATES.indexOf(state);

  const detail = $derived<Detail | null>(
    near ? { mark: near } : hovered && cursor !== null ? { run: hovered, at: cursor } : null,
  );

  /** Hours every 3 h, or every 6 h and more when the labels would touch. */
  const ticks = $derived.by(() => {
    const every = view
      ? (STEPS.find((step) => (step * w) / vSpan >= 52) ?? 6 * HOUR_MS)
      : 3 * HOUR_MS * Math.max(1, Math.ceil(52 / ((3 * HOUR_MS * w) / span)));
    const hour = new Intl.DateTimeFormat(lang(), { hour: "numeric" });
    const out: { x: number; label: string; anchor: string }[] = [];
    const first = day.start + Math.ceil((vFrom - day.start) / every) * every;
    for (let at = first; at <= vFrom + vSpan; at += every) {
      const px = x(at);
      out.push({
        x: px,
        label: every >= HOUR_MS ? t("ui.today.states.hour", { hour: hour.format(at) }) : formatTime(at),
        anchor: px < 1 ? "start" : px > w - 20 ? "end" : "middle",
      });
    }
    return out;
  });

  const date = $derived(
    new Intl.DateTimeFormat(lang(), { weekday: "long", day: "numeric", month: "long" }).format(day.start),
  );

  /** A day total, rounded to the minute: "45 min", "9 h 12". */
  function total(ms: number): string {
    const minutes = Math.round(ms / MINUTE_MS);
    return minutes === 0 ? "0 min" : formatDuration(minutes * MINUTE_MS);
  }

  const stateName = (state: PcState) => t(`ui.common.state.${state}`);
  const stateLower = (state: PcState) => stateName(state).toLocaleLowerCase(lang());

  const summary = $derived(
    t("ui.today.states.summary", {
      calm: total(day.totals.calm),
      busy: total(day.totals.busy),
      pressure: total(day.totals.pressure),
      suspended: total(day.totals.suspended),
    }) +
      " " +
      (day.occurrences.length
        ? tn("ui.today.states.summary.marks", day.occurrences.length)
        : t("ui.today.states.summary.no_marks")),
  );

  const filtered = $derived.by(() => {
    if (!filter) return null;
    const runs = day.runs.filter((r) => r.state === filter);
    const longest = runs.reduce<Run | null>((a, b) => (!a || b.to - b.from > a.to - a.from ? b : a), null);
    return { state: filter, count: runs.length, longest };
  });

  function value(at: number, column: "cpu" | "mem_used_pct" | "disk_active_max" | "gpu"): string {
    const point = pointAt(day.timeline, at);
    const v = point < 0 ? null : day.timeline[column][point];
    return v === null ? "—" : formatPercent(v);
  }

  function why(at: number): string {
    const reason = gapAt(day.gaps, at);
    return t(`ui.today.states.gap.${reason ?? "none"}`);
  }

  const live = $derived.by(() => {
    if (!detail) return "";
    if ("mark" in detail) {
      return t("ui.today.states.live.mark", {
        time: formatTime(detail.mark.start_ms),
        title: occurrenceTitle(detail.mark, level),
      });
    }
    const run = detail.run;
    return t("ui.today.states.live.run", {
      state: stateName(run.state),
      from: formatTime(run.from),
      to: formatTime(run.to),
      duration: total(run.to - run.from),
    });
  });

  function setFilter(state: PcState | null): void {
    filter = state === null || filter === state ? null : state;
  }

  function runAt(at: number): Run | null {
    const runs = day.runs.filter((r) => at >= r.from && at < r.to);
    return runs.find((r) => r.state === filter) ?? runs[0] ?? null;
  }

  function closest(marks: OccurrenceSummary[], at: (o: OccurrenceSummary) => number): OccurrenceSummary | null {
    return marks.reduce<OccurrenceSummary | null>((a, b) => (!a || at(b) < at(a) ? b : a), null);
  }

  /** A mark within a few pixels wins; otherwise the stretch drawn in the row under the pointer. */
  function hitTest(px: number, py: number): void {
    const at = vFrom + (px / w) * vSpan;
    const marks = day.occurrences.filter(
      (o) => Math.abs(x(o.start_ms) - px) < 7 && py >= TOP - 18 && py <= TOP + 3 * ROW,
    );
    near = closest(marks, (o) => Math.abs(x(o.start_ms) - px));
    cursor = near ? near.start_ms : Math.min(at, day.now - 1);
    hovered = null;
    if (near) return;
    const row = Math.floor((py - TOP) / ROW);
    const inRow = py - TOP - row * ROW;
    if (row < 0 || row >= STATES.length || inRow < 3 || inRow > ROW - 3) return;
    const slack = (4 / w) * vSpan;
    const run =
      day.runs.find((r) => r.state === STATES[row] && at >= r.from - slack && at < r.to + slack) ?? null;
    hovered = run;
  }

  /** Zooms by `factor` around the time under `px`, staying inside the day. */
  function zoom(factor: number, px: number): void {
    const at = vFrom + (px / w) * vSpan;
    const next = Math.max(MIN_VIEW_MS, vSpan * factor);
    if (next >= span) {
      view = null;
      return;
    }
    const from = Math.min(day.start + span - next, Math.max(day.start, at - (px / w) * next));
    view = { from, to: from + next };
  }

  /** Moves a zoomed view so it starts at `from`, staying inside the day. */
  function panTo(from: number): void {
    if (!view) return;
    const start = Math.min(day.start + span - vSpan, Math.max(day.start, from));
    view = { from: start, to: start + vSpan };
  }

  function onpointerdown(event: PointerEvent): void {
    if (!view || event.button !== 0 || !plot) return;
    event.preventDefault();
    plot.focus({ preventScroll: true });
    drag = { x: event.clientX, from: view.from, moved: false };
    plot.setPointerCapture(event.pointerId);
  }

  function onpointerup(): void {
    dragged = drag?.moved ?? false;
    drag = null;
  }

  function onpointermove(event: PointerEvent): void {
    if (!plot) return;
    if (drag) {
      const dx = event.clientX - drag.x;
      if (Math.abs(dx) > 3) drag.moved = true;
      if (drag.moved) {
        panTo(drag.from - (dx / w) * vSpan);
        clear();
        return;
      }
    }
    const box = plot.getBoundingClientRect();
    hitTest(Math.max(0, Math.min(box.width, event.clientX - box.left)), event.clientY - box.top);
  }

  function clear(): void {
    near = null;
    hovered = null;
  }

  function activate(): void {
    if (dragged) {
      dragged = false;
      return;
    }
    if (near) go(href.occurrence(near.id));
    else if (hovered) setFilter(hovered.state);
  }

  function moveTo(at: number): void {
    cursor = Math.max(day.start, Math.min(day.now - 1, at));
    const here = cursor;
    near = closest(
      day.occurrences.filter((o) => Math.abs(o.start_ms - here) <= MARK_NEAR_MS),
      (o) => Math.abs(o.start_ms - here),
    );
    hovered = near ? null : runAt(here);
    if (view && (here < view.from || here >= view.to)) panTo(here - vSpan / 2);
  }

  function onkeydown(event: KeyboardEvent): void {
    const step = (event.shiftKey ? 60 : 5) * MINUTE_MS;
    if (event.key === "ArrowRight") moveTo((cursor ?? day.start - step) + step);
    else if (event.key === "ArrowLeft") moveTo((cursor ?? day.now) - step);
    else if (event.key === "Home") moveTo(day.start);
    else if (event.key === "End") moveTo(day.now - 1);
    else if (event.key === "Enter") activate();
    else if (event.key === "+" || event.key === "=") zoom(0.8, cursor === null ? w / 2 : x(cursor));
    else if (event.key === "-" || event.key === "_") zoom(1.25, cursor === null ? w / 2 : x(cursor));
    else if (event.key === "0") view = null;
    else if (event.key === "Escape") {
      clear();
      cursor = null;
      setFilter(null);
    } else return;
    event.preventDefault();
  }

  // Ctrl and the wheel zoom around the pointer; Shift or a sideways wheel moves a zoomed view.
  $effect(() => {
    const el = plot;
    if (!el) return;
    const onwheel = (event: WheelEvent) => {
      if (event.ctrlKey) {
        event.preventDefault();
        zoom(event.deltaY < 0 ? 0.8 : 1.25, event.clientX - el.getBoundingClientRect().left);
      } else if (view && (event.shiftKey || Math.abs(event.deltaX) > Math.abs(event.deltaY))) {
        event.preventDefault();
        panTo(vFrom + ((event.shiftKey ? event.deltaY : event.deltaX) / w) * vSpan);
      }
    };
    el.addEventListener("wheel", onwheel, { passive: false });
    return () => el.removeEventListener("wheel", onwheel);
  });

  // A new day starts with the whole day on screen.
  $effect(() => {
    void day.start;
    view = null;
  });

  // The tooltip sits above the plot unless that would leave the scrolling area, and inside the card.
  $effect(() => {
    if (!detail || !tip || !plot || !card) return;
    const at = "mark" in detail ? x(detail.mark.start_ms) : x(detail.at);
    const box = plot.getBoundingClientRect();
    const outer = card.getBoundingClientRect();
    const tw = tip.offsetWidth;
    const left = Math.max(outer.left + 8 - box.left, Math.min(at - tw / 2, outer.right - 8 - box.left - tw));
    const limit = plot.closest(".content")?.getBoundingClientRect().top ?? 0;
    const above = box.top - tip.offsetHeight - 12 > limit;
    tip.style.left = `${left}px`;
    tip.style.top = above ? `${-tip.offsetHeight - 8}px` : `${plot.offsetHeight + 8}px`;
  });
</script>

<section class="card states" aria-labelledby="states-title" bind:this={card}>
  <div class="head">
    <h2 id="states-title">{t("ui.today.states.title")}</h2>
    <span class="date">
      {date} · {t("ui.today.states.range", {
        from: formatTime(view ? view.from : day.start),
        to: formatTime(view ? Math.min(view.to, day.now) : day.now),
      })}
      {#if view}
        <button type="button" class="clear" data-focus="whole-day" onclick={() => (view = null)}>
          {t("ui.today.states.zoom.reset")}
        </button>
      {/if}
    </span>
  </div>
  <div class="wrap">
    <div class="labels">
      {#each STATES as state (state)}
        <button
          type="button"
          class="legend"
          aria-pressed={filter === state}
          data-focus="legend-{state}"
          title={filter === state
            ? t("ui.today.states.show_all")
            : t("ui.today.states.only", { state: stateLower(state) })}
          onclick={() => setFilter(state)}
        >
          <span class="state-dot" style="background: var(--{STATE_TOKEN[state]})"></span>
          <span>{stateName(state)}</span>
          <b>{total(day.totals[state])}</b>
        </button>
      {/each}
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="plot"
      class:hit={detail !== null}
      class:pan={view !== null}
      role="application"
      tabindex="0"
      aria-label={t("ui.today.states.plot")}
      bind:this={plot}
      bind:clientWidth={width}
      {onpointerdown}
      {onpointermove}
      {onpointerup}
      onpointerleave={clear}
      onclick={activate}
      {onkeydown}
      onblur={clear}
    >
      <svg width={w} height={HEIGHT} viewBox="0 0 {w} {HEIGHT}" aria-hidden="true">
        {#each STATES as state, i (state)}
          <line class="guide" x1="0" x2={w} y1={TOP + i * ROW + ROW / 2} y2={TOP + i * ROW + ROW / 2} />
        {/each}
        {#each day.runs as run, j (run.from)}
          {@const x1 = x(run.from)}
          {@const x2 = x(run.to)}
          {@const y = TOP + rowOf(run.state) * ROW + 5}
          {@const next = day.runs[j + 1]}
          <rect
            class="run s-{run.state}"
            class:dim={filter !== null && filter !== run.state}
            class:on={hovered === run}
            x={x1}
            {y}
            width={Math.max(2, x2 - x1)}
            height={ROW - 10}
            rx={Math.min((ROW - 10) / 2, Math.max(1, (x2 - x1) / 2))}
          />
          {#if !filter && next && next.from === run.to}
            <line
              class="link"
              x1={x2}
              x2={x2}
              y1={y + (ROW - 10) / 2}
              y2={TOP + rowOf(next.state) * ROW + ROW / 2}
            />
          {/if}
        {/each}
        {#each ticks as tick (tick.x)}
          <text class="ax" x={tick.x} y={HEIGHT - 6} text-anchor={tick.anchor}>{tick.label}</text>
        {/each}
        {#each day.occurrences as o (o.id)}
          {@const mx = x(o.start_ms)}
          {@const on = near === o}
          <line
            class="mark t-{toneOf(o)}"
            x1={mx}
            x2={mx}
            y1={TOP - 4}
            y2={TOP + 3 * ROW}
            stroke-width={on ? 3.5 : 2}
          />
          <circle class="dot t-{toneOf(o)}" class:on cx={mx} cy={TOP - 8} r={on ? 8.5 : 6} />
        {/each}
      </svg>
      {#if detail}
        <div class="tip" role="presentation" bind:this={tip}>
          {#if "mark" in detail}
            {@const o = detail.mark}
            <div class="th">
              <span class="state-dot" style="background: var(--{toneOf(o)})"></span>
              <b>{formatTime(o.start_ms)}</b>
              <span>{occurrenceTitle(o, level)}</span>
            </div>
            <div class="sub">
              {kindName(o)} · {formatDuration(o.end_ms - o.start_ms)} · {causeName(o.cause)}
            </div>
            <div class="hint">{t("ui.today.states.hint.open")} · {t("ui.today.states.hint.zoom")}</div>
          {:else}
            {@const run = detail.run}
            <div class="th">
              <span class="state-dot" style="background: var(--{STATE_TOKEN[run.state]})"></span>
              <b>{stateName(run.state)}</b>
              <span class="muted">
                {t("ui.today.states.span", {
                  from: formatTime(run.from),
                  to: formatTime(run.to),
                  duration: total(run.to - run.from),
                })}
              </span>
            </div>
            {#if run.state === "suspended"}
              <div class="values">{why(detail.at)}</div>
            {:else}
              <div class="values">
                <span class="muted">{t("ui.today.states.at", { time: formatTime(detail.at) })}</span>
                <span>{t("ui.today.states.cpu")} <b>{value(detail.at, "cpu")}</b></span>
                <span>{t("ui.today.states.memory")} <b>{value(detail.at, "mem_used_pct")}</b></span>
                <span>{t("ui.today.states.disk")} <b>{value(detail.at, "disk_active_max")}</b></span>
                <span>{t("ui.today.states.gpu")} <b>{value(detail.at, "gpu")}</b></span>
              </div>
            {/if}
            <div class="hint">
              {filter === run.state
                ? t("ui.today.states.hint.show_all")
                : t("ui.today.states.hint.only", { state: stateLower(run.state) })}
              · {t("ui.today.states.hint.zoom")}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
  {#if filtered}
    <p class="filterline">
      <span class="state-dot" style="background: var(--{STATE_TOKEN[filtered.state]})"></span>
      <b>{stateName(filtered.state)}</b>:
      {filtered.longest
        ? t("ui.today.states.filter", {
            total: total(day.totals[filtered.state]),
            runs: tn("ui.today.states.runs", filtered.count),
            from: formatTime(filtered.longest.from),
            to: formatTime(filtered.longest.to),
          })
        : t("ui.today.states.filter.none")}
      <button type="button" class="clear" data-focus="show-all" onclick={() => setFilter(null)}>
        {t("ui.today.states.show_all")}
      </button>
    </p>
  {/if}
  <p class="sr" aria-live="polite">{live}</p>
  <p class="sr">{summary}</p>
</section>

<style>
  .states {
    container: states / inline-size;
    position: relative;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: baseline;
    gap: 4px 12px;
  }
  .head h2 {
    margin: 0;
  }
  .date {
    color: var(--ink2);
    font-size: 13px;
  }
  .wrap {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    gap: 12px;
    margin-top: 6px;
  }
  .labels {
    display: grid;
    grid-template-rows: repeat(4, 28px);
    align-content: start;
    padding-top: 22px;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    margin-left: -8px;
    padding: 0 8px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--ink2);
    font: 600 12.5px/1.35 var(--f);
    white-space: nowrap;
    text-align: left;
    cursor: pointer;
  }
  .legend b {
    margin-left: auto;
    color: var(--ink);
    font-weight: 600;
  }
  .legend:hover {
    background: var(--card2);
    color: var(--ink);
  }
  .legend[aria-pressed="true"] {
    background: var(--accent-t);
    color: var(--ink);
  }
  .plot {
    position: relative;
    cursor: default;
    border-radius: 8px;
  }
  .plot.hit {
    cursor: pointer;
  }
  .plot.pan {
    cursor: grab;
  }
  .plot.pan:active {
    cursor: grabbing;
  }
  /* Keyboard focus shows as the highlighted stretch and its tooltip, not as a ring. */
  .plot:focus-visible {
    outline: none;
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
  .run.s-pressure {
    fill: var(--press);
  }
  .run.s-busy {
    fill: var(--busy);
  }
  .run.s-calm {
    fill: var(--calm);
  }
  .run.s-suspended {
    fill: var(--susp);
  }
  .run.dim {
    opacity: 0.14;
  }
  .run.on {
    stroke: var(--ink);
    stroke-width: 2;
  }
  .link {
    stroke: var(--ink3);
    stroke-opacity: 0.3;
  }
  .ax {
    fill: var(--ink3);
    font: 500 11.5px/1 var(--f);
  }
  .mark {
    stroke-linecap: round;
  }
  .mark.t-busy {
    stroke: var(--busy);
  }
  .mark.t-press {
    stroke: var(--press);
  }
  .mark.t-freeze {
    stroke: var(--freeze);
  }
  .dot {
    stroke: var(--card);
    stroke-width: 2.5;
  }
  .dot.on {
    stroke: var(--ink);
    stroke-width: 2;
  }
  .dot.t-busy {
    fill: var(--busy);
  }
  .dot.t-press {
    fill: var(--press);
  }
  .dot.t-freeze {
    fill: var(--freeze);
  }
  .tip {
    position: absolute;
    z-index: 3;
    width: max-content;
    min-width: min(240px, 100cqw - 16px);
    max-width: min(340px, 100cqw - 16px);
    padding: 10px 12px;
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: 12px;
    box-shadow: var(--shadow-float);
    font-size: 13px;
    line-height: 1.45;
    pointer-events: none;
  }
  .th {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    font-size: 15px;
    line-height: 1.3;
  }
  .th b {
    font-weight: 600;
  }
  .sub {
    margin-top: 2px;
    color: var(--ink2);
  }
  .values {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-top: 8px;
    color: var(--ink2);
  }
  .values b {
    color: var(--ink);
    font-weight: 600;
  }
  .hint {
    margin-top: 8px;
    color: var(--ink3);
    font-size: 12.5px;
  }
  .filterline {
    margin: 10px 0 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .filterline .state-dot {
    margin-right: 6px;
  }
  .filterline b {
    color: var(--ink);
  }
  .clear {
    padding: 0 4px;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--accent);
    font-weight: 600;
    cursor: pointer;
  }
  @container states (max-width: 520px) {
    .wrap {
      grid-template-columns: minmax(0, 1fr);
    }
    .labels {
      grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
      grid-template-rows: none;
      gap: 2px 12px;
      padding-top: 0;
    }
    .legend {
      margin-left: 0;
    }
  }
  @media (forced-colors: active) {
    .run.s-pressure,
    .run.s-busy,
    .run.s-calm,
    .run.s-suspended {
      fill: CanvasText;
    }
    .run.on {
      stroke: Highlight;
    }
    .mark.t-busy,
    .mark.t-press,
    .mark.t-freeze {
      stroke: Highlight;
    }
    .dot.t-busy,
    .dot.t-press,
    .dot.t-freeze {
      fill: Highlight;
      stroke: Canvas;
    }
    .dot.on {
      stroke: CanvasText;
    }
    .guide,
    .link {
      stroke: GrayText;
    }
    .ax {
      fill: CanvasText;
    }
    .legend[aria-pressed="true"] {
      outline: 2px solid Highlight;
    }
    .tip {
      border-color: CanvasText;
    }
  }
</style>
