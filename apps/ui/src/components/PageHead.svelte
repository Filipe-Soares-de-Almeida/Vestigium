<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    sub,
    meta,
    actions,
  }: {
    title: string;
    /** One line under the title, as plain text. */
    sub?: string;
    /** The line under the title when it holds more than text, such as a status symbol. */
    meta?: Snippet;
    /** Buttons at the right of the title. */
    actions?: Snippet;
  } = $props();
</script>

<header class="head">
  <div class="text">
    <h1>{title}</h1>
    {#if meta}
      <p class="sub">{@render meta()}</p>
    {:else if sub}
      <p class="sub">{sub}</p>
    {/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px 24px;
    margin: 4px 0 0;
  }
  .text {
    min-width: 0;
  }
  .sub {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    margin: 6px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  /* The first block under the header opens without a divider. */
  .head + :global(.block) {
    margin-top: 24px;
    padding-top: 0;
    border-top: 0;
  }
</style>
