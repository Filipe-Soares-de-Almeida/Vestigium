<script lang="ts">
  /** A small line over time with its caption: the values in order, the two ends of the span named. */
  let {
    values,
    at,
    label,
    from,
    to,
  }: {
    values: number[];
    /** When each value was read; without it the values are evenly spaced. */
    at?: number[];
    /** What the line shows, also its accessible name. */
    label: string;
    from: string;
    to: string;
  } = $props();

  const W = 600;
  const H = 96;

  const line = $derived.by(() => {
    if (values.length < 2) return null;
    const min = Math.min(...values);
    const span = Math.max(...values) - min;
    const first = at?.[0] ?? 0;
    const width = at ? at[at.length - 1] - first || 1 : values.length - 1;
    const x = (i: number) => (((at ? at[i] - first : i) / width) * W).toFixed(1);
    // A value that never moves draws at mid height.
    const y = (v: number) => (span ? H - 8 - ((v - min) / span) * (H - 20) : H / 2).toFixed(1);
    return values.map((v, i) => `${i ? "L" : "M"}${x(i)} ${y(v)}`).join("");
  });
</script>

<div class="spark">
  {#if line}
    <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" role="img" aria-label={label}>
      <path d="{line}L{W} {H}L0 {H}Z" class="area" />
      <path d={line} class="stroke" />
    </svg>
  {/if}
  <p class="axis"><span>{from}</span><span class="what">{label}</span><span>{to}</span></p>
</div>

<style>
  .spark {
    margin: 4px 0 0;
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
