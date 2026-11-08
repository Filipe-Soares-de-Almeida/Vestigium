<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    first = false,
    actions,
    children,
  }: {
    title: string;
    /** The block right under the page header has no divider above it. */
    first?: boolean;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<section class="block" class:first>
  <header>
    <h2>{title}</h2>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </header>
  {@render children()}
</section>

<style>
  .block {
    min-width: 0;
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--rule);
  }
  .block.first {
    margin-top: 24px;
    padding-top: 0;
    border-top: 0;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 20px;
    min-height: 30px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font: 400 20px/1.3 var(--fd);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
  }
</style>
