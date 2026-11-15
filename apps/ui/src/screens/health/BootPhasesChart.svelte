<script lang="ts">
  import ChartTip from "../../components/ChartTip.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { BootDto } from "../../lib/types";
  import { wheelZoom, Zoom } from "../../lib/zoom.svelte";
  import { formatSeconds, judge, PHASES, phaseMs, rangeText, startupWhen, valueText, waterfall, waterfallCeiling } from "./startup";

  /** The phases of one startup, a row each, with the bar of every phase from where it began to where it ended; zooms like the other time charts. */
  let {
    boot,
    zoom: exposed = $bindable(),
    plot: plotEl = $bindable(),
  }: {
    boot: BootDto;
    /** The chart's zoom, for a ZoomBar placed in the block header. */
    zoom?: Zoom;
    /** The plot element, which gets focus back when the zoom is reset. */
    plot?: HTMLElement;
  } = $props();

  const ROW = 32;
  const BAR = 14;
  /** Room above the first row for the name of the TPM line. */
  const TOP = 22;
  const MIN_SPAN = 5;
  const STEPS = [1, 2, 5, 10, 15, 20, 30, 60, 120, 300, 600];
  const k = (key: string, args?: Record<string, string | number>) => t(`ui.health.startup.${key}`, args);
  const seconds = (s: number) => formatSeconds(s * 1000);

  let width = $state(0);
  /** The row the readout is on; `null` when none. */
  let cur = $state<number | null>(null);
  let said = $state("");

  const wf = $derived(waterfall(boot));
  const ceiling = $derived(waterfallCeiling(wf));

  const zoom = new Zoom({
    bounds: () => ({ from: 0, to: ceiling }),
    width: () => width,
    get minSpan() {
      return Math.min(MIN_SPAN, ceiling);
    },
  });
  exposed = zoom;

  const lo = $derived(zoom.from);
  const span = $derived(zoom.span || 1);
  /** Share of the axis, in percent, at second `v`. */
  const pct = (v: number) => ((v - lo) / span) * 100;

  const rows = $derived(
    PHASES.map((p, i) => {
      const seg = wf.segments[i];
      const placed = seg.from !== null && seg.to !== null;
      const before = Math.max(0, ...wf.segments.slice(0, i).map((s) => s.to ?? 0));
      return { p, seg, placed, ms: phaseMs(boot, p.id), verdict: judge(boot, p.id), before };
    }),
  );

  const ticks = $derived.by(() => {
    const want = Math.max(3, Math.floor((width || 600) / 90));
    const step = STEPS.find((s) => span / s <= want) ?? STEPS[STEPS.length - 1];
    const out: number[] = [];
    for (let v = Math.ceil(lo / step - 1e-9) * step; v <= lo + span + 1e-9; v += step) out.push(v);
    return out;
  });

  const summary = $derived(rows.map((r) => `${k(`phase.${r.p.id}`)} ${valueText(r.ms)}`).join(", "));
  const label = $derived(t("ui.health.startup.boot.chart", { when: startupWhen(boot.kernel_start_ms) }));
  const tpmShown = $derived(wf.tpmAt !== null && wf.tpmAt >= lo && wf.tpmAt <= lo + span);

  /** The readout of one row: its length, its place on the axis, what is normal and what the person should know. */
  function tipRows(i: number): { key: string; label: string; value: string }[] {
    const r = rows[i];
    if (!r.placed || r.ms === null) return [{ key: "none", label: k("body.unmeasured"), value: "" }];
    const out = [
      { key: "ms", label: t("ui.health.startup.boot.tip.duration"), value: formatSeconds(r.ms) },
      { key: "span", label: t("ui.health.startup.boot.tip.span", { from: seconds(r.seg.from!), to: seconds(r.seg.to!) }), value: "" },
      { key: "range", label: rangeText(r.p.id), value: "" },
    ];
    if (r.p.id === "profile" && boot.tpm_wait_ms !== null && boot.tpm_wait_ms > 0) {
      out.push({ key: "tpm", label: t("ui.health.startup.boot.tip.tpm"), value: formatSeconds(boot.tpm_wait_ms) });
    }
    if (r.p.id === "signin" && boot.probable_wait) out.push({ key: "wait", label: k("word.wait"), value: "" });
    return out;
  }

  function readout(i: number): string {
    const r = rows[i];
    return `${k(`phase.${r.p.id}`)}: ${valueText(r.ms)}${r.p.id === "signin" && boot.probable_wait ? `, ${k("word.wait")}` : ""}.`;
  }

  /** Where the readout points: the end of the bar, or near the left when the phase has none. */
  function anchor(i: number): number {
    const to = rows[i].seg.to;
    return to === null ? 20 : Math.min(width, Math.max(0, (pct(to) / 100) * width));
  }

  function move(i: number): void {
    cur = i;
    const { from, to } = rows[i].seg;
    if (from !== null && to !== null) zoom.reveal((from + to) / 2);
    said = readout(i);
  }

  function onkeydown(event: KeyboardEvent): void {
    const last = rows.length - 1;
    const now = cur ?? 2;
    const next =
      event.key === "ArrowUp" || event.key === "ArrowLeft" ? Math.max(0, now - 1)
      : event.key === "ArrowDown" || event.key === "ArrowRight" ? Math.min(last, now + 1)
      : event.key === "Home" ? 0
      : event.key === "End" ? last
      : null;
    if (next !== null) {
      event.preventDefault();
      move(next);
    } else if (zoom.key(event.key, cur !== null && rows[cur].placed ? anchor(cur) : width / 2)) {
      event.preventDefault();
    } else if (event.key === "Escape") {
      cur = null;
    }
  }

  function onpointermove(event: PointerEvent & { currentTarget: HTMLElement }): void {
    if (zoom.drag(event)) return;
    const row = Math.floor((event.clientY - event.currentTarget.getBoundingClientRect().top - TOP) / ROW);
    cur = row >= 0 && row < rows.length ? row : null;
  }
</script>

<div class="wf" style:--rh="{ROW}px" style:--th="{TOP}px">
  <div class="labels" aria-hidden="true">
    {#each rows as r, i (r.p.id)}<span class="lab" class:on={cur === i}>{k(`phase.${r.p.id}`)}</span>{/each}
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="plot"
    class:pan={zoom.view !== null}
    style:height="{TOP + rows.length * ROW}px"
    tabindex="0"
    role="application"
    aria-label={label}
    bind:this={plotEl}
    bind:clientWidth={width}
    use:wheelZoom={zoom}
    onpointerdown={(event) => zoom.press(event, event.currentTarget)}
    {onpointermove}
    onpointerup={() => zoom.release()}
    onpointercancel={() => zoom.release()}
    onpointerleave={() => (cur = null)}
    onfocus={() => (cur ??= 2)}
    onblur={() => (cur = null)}
    {onkeydown}
  >
    <div class="field">
      {#each ticks as v (v)}<span class="grid" style:left="{pct(v)}%"></span>{/each}
      {#if cur !== null}<span class="band" style:top="{TOP + cur * ROW}px"></span>{/if}
      {#each rows as r, i (r.p.id)}
        {#if r.placed}
          <span
            class="bar"
            class:att={r.verdict.kind === "attention"}
            class:wait={r.p.id === "signin" && boot.probable_wait}
            class:on={cur === i}
            style:top="{TOP + i * ROW + (ROW - BAR) / 2}px"
            style:left="{pct(r.seg.from!)}%"
            style:width="{((r.seg.to! - r.seg.from!) / span) * 100}%"
          ></span>
        {:else}
          <span class="none" style:top="{TOP + i * ROW + (ROW - 18) / 2}px" style:left="calc({Math.min(80, Math.max(0, pct(r.before)))}% + 6px)">{k("value.unmeasured")}</span>
        {/if}
      {/each}
      {#if tpmShown}
        <span class="tpm" class:flip={pct(wf.tpmAt!) > 80} style:left="{pct(wf.tpmAt!)}%"><span>{t("ui.health.startup.boot.tpm.line")}</span></span>
      {/if}
    </div>
    {#if cur !== null}
      <ChartTip left={anchor(cur)} bound={width} heading={k(`phase.${rows[cur].p.id}`)} rows={tipRows(cur)} top={TOP + (cur + 1) * ROW} />
    {/if}
  </div>
  <div class="axis" aria-hidden="true">
    {#each ticks as v (v)}
      {@const at = pct(v)}
      <span class:first={at < 2} class:last={at > 98} style:left="{at}%">{v} s</span>
    {/each}
  </div>
  <p class="cap">{wf.fromWindows ? t("ui.health.startup.boot.axis.windows") : t("ui.health.startup.boot.axis.power")}</p>
  <p class="sr">{summary}.</p>
  <p class="sr" aria-live="polite">{said}</p>
</div>

<style>
  .wf {
    display: grid;
    grid-template-columns: 176px minmax(0, 1fr);
    margin: 4px 0 0;
  }
  .labels {
    display: grid;
    grid-auto-rows: var(--rh);
    padding-top: var(--th);
    padding-right: 12px;
  }
  .lab {
    display: flex;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.2;
  }
  .lab.on {
    color: var(--ink);
    font-weight: 600;
  }
  .plot {
    position: relative;
    touch-action: pan-y;
  }
  .plot:focus-visible {
    outline: none;
  }
  .plot.pan {
    cursor: grab;
  }
  .field {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }
  .grid {
    position: absolute;
    top: var(--th);
    bottom: 0;
    width: 1px;
    background: var(--stroke2);
  }
  .band {
    position: absolute;
    right: 0;
    left: 0;
    height: var(--rh);
    border-radius: 6px;
    background: var(--row-hover);
  }
  .bar {
    --c: var(--accent);
    position: absolute;
    height: 14px;
    min-width: 3px;
    border-radius: 7px;
    background: var(--c);
  }
  .bar.att {
    --c: color-mix(in srgb, var(--busy) 78%, var(--ink));
  }
  .bar.wait {
    background: repeating-linear-gradient(135deg, var(--c) 0 3px, transparent 3px 7px);
    box-shadow: inset 0 0 0 1px var(--c);
  }
  .bar.on {
    box-shadow: 0 0 0 2px var(--main), 0 0 0 3.5px var(--ink);
  }
  .bar.wait.on {
    box-shadow:
      inset 0 0 0 1px var(--c),
      0 0 0 2px var(--main),
      0 0 0 3.5px var(--ink);
  }
  .none {
    position: absolute;
    height: 18px;
    padding: 0 8px;
    border-radius: 999px;
    background: var(--row);
    color: var(--ink3);
    font-size: 12px;
    line-height: 18px;
    white-space: nowrap;
  }
  .tpm {
    position: absolute;
    top: 14px;
    bottom: 0;
    border-left: 1px dashed var(--ink3);
  }
  .tpm span {
    position: absolute;
    bottom: 100%;
    left: 4px;
    padding-bottom: 1px;
    color: var(--ink3);
    font-size: 11px;
    line-height: 12px;
    white-space: nowrap;
  }
  .tpm.flip span {
    right: 4px;
    left: auto;
  }
  .axis {
    position: relative;
    grid-column: 2;
    height: 18px;
    margin-top: 4px;
    color: var(--ink2);
    font-size: 12px;
  }
  .axis span {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    white-space: nowrap;
  }
  .axis .first {
    transform: none;
  }
  .axis .last {
    transform: translateX(-100%);
  }
  .cap {
    grid-column: 2;
    margin: 2px 0 0;
    color: var(--ink2);
    font-size: 12px;
  }
  @container content (max-width: 640px) {
    .wf {
      grid-template-columns: 112px minmax(0, 1fr);
    }
  }
</style>
