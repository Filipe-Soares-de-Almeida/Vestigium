<script lang="ts">
  /** A 96 x 28 line of readings from zero up to `ceiling`; a `null` breaks the line. Decorative: the text beside it says the same. */
  let { values, ceiling }: { values: (number | null)[]; ceiling: number } = $props();

  const W = 96;
  const H = 28;

  const marks = $derived.by(() => {
    const n = values.length;
    const x = (i: number) => (n > 1 ? 2 + (i / (n - 1)) * (W - 4) : W / 2);
    const y = (v: number) => H - 3 - (Math.min(v, ceiling) / ceiling) * (H - 6);
    const runs: number[][] = [];
    let cut = true;
    values.forEach((v, i) => {
      if (v !== null) {
        if (cut) runs.push([i]);
        else runs[runs.length - 1].push(i);
      }
      cut = v === null;
    });
    const point = (i: number) => `${x(i).toFixed(1)} ${y(values[i] as number).toFixed(1)}`;
    const drawn = runs.filter((run) => run.length > 1);
    return {
      line: drawn.map((run) => run.map((i, k) => `${k ? "L" : "M"}${point(i)}`).join("")).join(""),
      alone: runs.filter((run) => run.length === 1).map((run) => ({ x: x(run[0]), y: y(values[run[0]] as number) })),
    };
  });
</script>

<svg width={W} height={H} viewBox="0 0 {W} {H}" aria-hidden="true" focusable="false">
  <line class="base" x1="0" x2={W} y1={H - 2.5} y2={H - 2.5} />
  <path class="line" d={marks.line} />
  {#each marks.alone as p, i (i)}<circle class="pt" cx={p.x} cy={p.y} r="1.8" />{/each}
</svg>

<style>
  svg {
    display: block;
    overflow: hidden;
  }
  .base {
    stroke: var(--stroke);
  }
  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-linejoin: round;
  }
  .pt {
    fill: var(--accent);
  }
</style>
