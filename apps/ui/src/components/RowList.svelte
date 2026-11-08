<script lang="ts">
  import type { Snippet } from "svelte";

  /** A stack of RowCard rows, 4 px apart. */
  let { children, label }: { children: Snippet; label?: string } = $props();
</script>

<ul class="rows" aria-label={label}>
  {@render children()}
</ul>

<style>
  .rows {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
