<script lang="ts">
  import type { Snippet } from "svelte";

  /** The line of facts under a detail title: a label and a value each. */
  let { rows }: { rows: { label: string; value: string | Snippet }[] } = $props();
</script>

<dl class="facts">
  {#each rows as row (row.label)}
    <div>
      <dt>{row.label}</dt>
      <dd>
        {#if typeof row.value === "string"}{row.value}{:else}{@render row.value()}{/if}
      </dd>
    </div>
  {/each}
</dl>

<style>
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 40px;
    margin: 18px 0 0;
  }
  .facts div {
    display: grid;
    gap: 2px;
  }
  dt {
    color: var(--ink2);
    font-size: 12.5px;
  }
  dd {
    margin: 0;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }
</style>
