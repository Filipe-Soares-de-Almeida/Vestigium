<script lang="ts">
  import type { Snippet } from "svelte";

  // One setting: its name and description, and the control; a wide row puts the control below.
  let {
    id,
    title,
    hint,
    wide = false,
    children,
  }: { id: string; title?: string; hint?: string; wide?: boolean; children: Snippet } = $props();
</script>

<div class="set" class:wide>
  {#if title}
    <div class="text">
      <p class="name" id="{id}-name">{title}</p>
      {#if hint}<p class="hint" id="{id}-hint">{hint}</p>{/if}
    </div>
  {/if}
  <div class="control">{@render children()}</div>
</div>

<style>
  .set {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px 20px;
    align-items: center;
    min-height: 66px;
    padding: 14px 18px;
    background: var(--card);
    border: 1px solid var(--stroke2);
    border-radius: 8px;
  }
  .wide {
    grid-template-columns: minmax(0, 1fr);
  }
  p {
    margin: 0;
  }
  .name {
    font-weight: 600;
    line-height: 1.3;
  }
  .hint {
    max-width: 62ch;
    margin-top: 2px;
    color: var(--ink2);
    font: 400 13px/1.4 var(--f);
  }
  .control {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    justify-content: flex-end;
  }
  .wide .text + .control {
    margin-top: 8px;
  }
  .wide .control {
    display: block;
  }
  @container content (max-width: 720px) {
    .set {
      grid-template-columns: minmax(0, 1fr);
    }
    .control {
      justify-content: flex-start;
    }
  }
  @media (forced-colors: active) {
    .set {
      border-color: CanvasText;
    }
  }
</style>
