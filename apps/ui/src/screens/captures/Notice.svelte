<script lang="ts">
  import type { Snippet } from "svelte";

  import type { IconName } from "../../components/Icon.svelte";
  import { focusHere } from "../../lib/focus";

  let {
    id,
    icon,
    title,
    body,
    children,
  }: { id: string; icon: IconName; title: string; body: string; children?: Snippet } = $props();
</script>

<section class="notice" aria-labelledby="{id}-title" data-icon={icon}>
  <div>
    <h2 id="{id}-title" tabindex="-1" use:focusHere>{title}</h2>
    <p>{body}</p>
    {#if children}
      <div class="actions">{@render children()}</div>
    {/if}
  </div>
</section>

<style>
  .notice {
    margin-top: 24px;
  }
  h2 {
    margin: 0 0 8px;
    font: 400 20px/1.3 var(--fd);
    outline: none;
  }
  p {
    margin: 0 0 12px;
    color: var(--ink2);
    line-height: 1.5;
    max-width: 62ch;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
