<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon, { type IconName } from "../../components/Icon.svelte";
  import { focusHere } from "../../lib/focus";

  let {
    id,
    icon,
    title,
    body,
    children,
  }: { id: string; icon: IconName; title: string; body: string; children?: Snippet } = $props();
</script>

<section class="card notice" aria-labelledby="{id}-title">
  <span class="icochip"><Icon name={icon} /></span>
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
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 14px;
    align-items: start;
  }
  .icochip {
    background: var(--card2);
    border: 1px solid var(--stroke2);
    color: var(--ink2);
  }
  h2 {
    margin: 2px 0 6px;
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
