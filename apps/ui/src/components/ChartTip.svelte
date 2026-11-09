<script lang="ts" module>
  /** One line of the readout: a lane's name and its value at the point. */
  export interface TipRow {
    key: string;
    label: string;
    value: string;
    /** CSS colour of the dot before the name; none when absent. */
    dot?: string;
  }
</script>

<script lang="ts">
  let {
    left,
    bound,
    heading,
    rows,
    top = 4,
  }: {
    /** Pixels from the left of the plot to the point being read. */
    left: number;
    /** Width of the plot; the readout flips to the left of the point before passing it. */
    bound: number;
    heading: string;
    rows: TipRow[];
    top?: number;
  } = $props();

  let width = $state(0);
  const x = $derived(left + 14 + width > bound ? left - 14 - width : left + 14);
</script>

<div class="tip" style:left="{x}px" style:top="{top}px" bind:clientWidth={width} aria-hidden="true">
  <b>{heading}</b>
  <div class="rows">
    {#each rows as row (row.key)}
      <span>
        {#if row.dot}<span class="state-dot" style:background={row.dot}></span>{/if}
        {row.label}
        <b>{row.value}</b>
      </span>
    {/each}
  </div>
</div>

<style>
  .tip {
    position: absolute;
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
  @media (forced-colors: active) {
    .tip {
      border-color: CanvasText;
    }
  }
</style>
