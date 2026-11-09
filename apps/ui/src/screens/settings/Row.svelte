<script lang="ts">
  import type { Snippet } from "svelte";

  // One setting as a section: its title, the control and its note; `wide` puts the note above the control.
  let {
    id,
    title,
    label,
    hint,
    wide = false,
    children,
  }: { id: string; title?: string; label?: string; hint?: string; wide?: boolean; children: Snippet } = $props();
</script>

<section class="sec" class:wide aria-labelledby={title ? (label ? `${id}-title` : `${id}-name`) : undefined}>
  {#if title}
    <header><h2 id={label ? `${id}-title` : `${id}-name`}>{title}</h2></header>
  {/if}
  {#if label}<p class="fld-l" id="{id}-name">{label}</p>{/if}
  {#if wide && hint}<p class="p lead" id="{id}-hint">{hint}</p>{/if}
  <div class="control">{@render children()}</div>
  {#if !wide && hint}<p class="fld-n" id="{id}-hint">{hint}</p>{/if}
</section>

<style>
  .sec {
    min-width: 0;
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--rule);
  }
  header {
    display: flex;
    align-items: center;
    min-height: 30px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
  }
  .control {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 16px;
    align-items: center;
  }
  .wide .control {
    display: block;
  }
</style>
