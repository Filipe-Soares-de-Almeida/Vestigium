<script lang="ts">
  import type { Snippet } from "svelte";

  import { focusHere } from "../../lib/focus";

  let { id, title, body, children }: { id: string; title: string; body: string; children?: Snippet } = $props();
</script>

<section class="notice" aria-labelledby="{id}-title">
  <p class="title" id="{id}-title" tabindex="-1" use:focusHere>{title}</p>
  <p class="p">{body}</p>
  {#if children}
    <div class="acts">{@render children()}</div>
  {/if}
</section>

<style>
  .title {
    margin: 0 0 8px;
    font: 600 16px/1.35 var(--f);
    outline: none;
  }
</style>
