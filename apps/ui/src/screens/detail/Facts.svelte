<script lang="ts">
  import type { Snippet } from "svelte";

  /** The line of facts under a detail title: a label and a value each. */
  let { rows, dim = false }: { rows: { label: string; value: string | Snippet }[]; dim?: boolean } = $props();
</script>

<dl class="facts" class:veiled={dim}>
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
    gap: 12px 52px;
    margin: 22px 0 0;
  }
  .facts div {
    display: grid;
    gap: 3px;
  }
  dt {
    color: var(--ink2);
    font-size: 12.5px;
  }
  dd {
    display: flex;
    align-items: center;
    margin: 0;
    font-size: 15px;
  }
  dd :global(.status) {
    color: var(--ink);
    font-size: 15px;
  }
</style>
