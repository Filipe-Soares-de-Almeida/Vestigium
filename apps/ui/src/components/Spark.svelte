<script lang="ts">
  import { formatClock, formatNumber } from "../lib/format";
  import { wheelZoom, Zoom } from "../lib/zoom.svelte";
  import ChartTip from "./ChartTip.svelte";

  /** A small line over time with its caption; it reads out a point under the pointer or the arrows. */
  let {
    values,
    at,
    label,
    from,
    to,
    format = formatNumber,
    when = formatClock,
  }: {
    values: number[];
    /** When each value was read; without it the values are evenly spaced. */
    at?: number[];
    /** What the line shows, also its accessible name and the readout's row. */
    label: string;
    from: string;
    to: string;
    /** How the readout writes a value. */
    format?: (value: number) => string;
    /** How the readout and a zoomed axis write a moment of `at`. */
    when?: (at: number) => string;
  } = $props();

  const W = 600;
  const H = 96;
  /** A step this many times the usual one is a gap, where the line breaks. */
  const GAP_STEPS = 3;

  let width = $state(0);
  /** The moment the readout is on; the reading nearest it is shown, so a refresh does not move it. */
  let cursorAt = $state<number | null>(null);
  let said = $state("");

  const xs = $derived(at ?? values.map((_, i) => i));
  /** The usual distance between two readings. */
  const step = $derived.by(() => {
    const steps = xs
      .slice(1)
      .map((x, i) => x - xs[i])
      .sort((a, b) => a - b);
    return steps.length ? steps[Math.floor(steps.length / 2)] || 1 : 1;
  });

  const zoom = new Zoom({
    bounds: () => ({ from: xs[0] ?? 0, to: xs[xs.length - 1] ?? 1 }),
    width: () => width,
    get minSpan() {
      return step * 6;
    },
  });

  const lo = $derived(zoom.from);
  const span = $derived(zoom.span || 1);
  /** The readings on screen, with one more at each side so the line reaches the edges. */
  const shown = $derived.by(() => {
    const inside = xs.flatMap((x, i) => (x >= lo && x <= lo + span ? [i] : []));
    if (!inside.length) return [];
    const first = Math.max(0, inside[0] - 1);
    const last = Math.min(xs.length - 1, inside[inside.length - 1] + 1);
    return Array.from({ length: last - first + 1 }, (_, k) => first + k);
  });
  const scale = $derived.by(() => {
    const seen = shown.map((i) => values[i]);
    const min = Math.min(...seen);
    return { min, span: Math.max(...seen) - min };
  });

  const cursor = $derived(cursorAt === null || !xs.length ? null : indexNear(cursorAt));

  function indexNear(x: number): number {
    return xs.reduce((best, v, i) => (Math.abs(v - x) < Math.abs(xs[best] - x) ? i : best), 0);
  }

  const xOf = (i: number) => ((xs[i] - lo) / span) * W;
  // A value that never moves draws at mid height.
  const yOf = (v: number) => (scale.span ? H - 8 - ((v - scale.min) / scale.span) * (H - 20) : H / 2);

  /** The line in stretches with no gap, and the area under each. */
  const paths = $derived.by(() => {
    const runs: number[][] = [];
    for (const i of shown) {
      const run = runs[runs.length - 1];
      if (run && !(at && xs[i] - xs[run[run.length - 1]] > step * GAP_STEPS)) run.push(i);
      else runs.push([i]);
    }
    const drawn = runs.filter((run) => run.length > 1);
    const point = (i: number) => `${xOf(i).toFixed(1)} ${yOf(values[i]).toFixed(1)}`;
    const line = drawn.map((run) => run.map((i, k) => `${k ? "L" : "M"}${point(i)}`).join("")).join("");
    const area = drawn
      .map((run) => `M${xOf(run[0]).toFixed(1)} ${H}${run.map((i) => `L${point(i)}`).join("")}L${xOf(run[run.length - 1]).toFixed(1)} ${H}Z`)
      .join("");
    return { line, area };
  });

  /** The reading nearest the pointer, among those on screen. */
  function nearest(px: number): number | null {
    if (!shown.length || width <= 0) return null;
    const target = lo + (px / width) * span;
    return shown.reduce((best, i) => (Math.abs(xs[i] - target) < Math.abs(xs[best] - target) ? i : best), shown[0]);
  }

  function readout(i: number): string {
    return `${when(xs[i])}: ${label} ${format(values[i])}`;
  }

  function onkeydown(event: KeyboardEvent): void {
    const last = values.length - 1;
    if (last < 0) return;
    const now = cursor ?? last;
    const next =
      event.key === "ArrowLeft" ? Math.max(0, now - 1)
      : event.key === "ArrowRight" ? Math.min(last, now + 1)
      : event.key === "Home" ? 0
      : event.key === "End" ? last
      : null;
    if (next !== null) {
      event.preventDefault();
      cursorAt = xs[next];
      zoom.reveal(xs[next]);
      said = readout(next);
    } else if (zoom.key(event.key, cursor !== null ? ((xs[cursor] - lo) / span) * width : width / 2)) {
      event.preventDefault();
    } else if (event.key === "Escape") {
      cursorAt = null;
    }
  }
</script>

<div class="spark">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="plot"
    class:pan={zoom.view !== null}
    tabindex="0"
    role="application"
    aria-label={label}
    bind:clientWidth={width}
    use:wheelZoom={zoom}
    onpointerdown={(event) => zoom.press(event, event.currentTarget)}
    onpointermove={(event) => {
      if (zoom.drag(event)) return;
      const i = nearest(event.clientX - event.currentTarget.getBoundingClientRect().left);
      cursorAt = i === null ? null : xs[i];
    }}
    onpointerup={() => zoom.release()}
    onpointercancel={() => zoom.release()}
    onpointerleave={() => (cursorAt = null)}
    onfocus={() => (cursorAt ??= xs.length ? xs[xs.length - 1] : null)}
    onblur={() => (cursorAt = null)}
    {onkeydown}
  >
    <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
      <path d={paths.area} class="area" />
      <path d={paths.line} class="stroke" />
    </svg>
    {#if cursor !== null && cursor < values.length}
      {@const px = ((xs[cursor] - lo) / span) * width}
      <span class="rule" style:left="{px}px"></span>
      <span class="dot" style:left="{px}px" style:top="{yOf(values[cursor])}px"></span>
      <ChartTip left={px} bound={width} heading={when(xs[cursor])} rows={[{ key: "v", label, value: format(values[cursor]) }]} />
    {/if}
  </div>
  <p class="axis">
    <span>{zoom.view ? when(lo) : from}</span><span class="what">{label}</span><span>{zoom.view ? when(lo + span) : to}</span>
  </p>
  <p class="sr" aria-live="polite">{said}</p>
</div>

<style>
  .spark {
    margin: 4px 0 0;
  }
  .plot {
    position: relative;
    height: 96px;
    touch-action: pan-y;
  }
  .plot:focus-visible {
    outline: none;
  }
  .plot.pan {
    cursor: grab;
  }
  svg {
    display: block;
    width: 100%;
    height: 96px;
  }
  .area {
    fill: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .stroke {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
  .rule {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--ink3);
    pointer-events: none;
  }
  .dot {
    position: absolute;
    width: 9px;
    height: 9px;
    margin: -4.5px 0 0 -4.5px;
    border: 2px solid var(--accent);
    border-radius: 50%;
    background: var(--main);
    pointer-events: none;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin: 6px 0 0;
    color: var(--ink2);
    font-size: 12px;
  }
  .what {
    color: var(--ink);
    text-align: center;
  }
</style>
