<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import type { Zoom } from "../lib/zoom.svelte";

  let {
    zoom,
    range,
    plot,
    hint,
  }: {
    zoom: Zoom;
    /** The stretch on screen, worded for the reader; shown while zoomed. */
    range: string;
    /** The chart's plot, which gets focus back when the button that had it goes away. */
    plot?: HTMLElement;
    /** Replaces the usual hint, for a chart with more gestures. */
    hint?: string;
  } = $props();

  function reset(): void {
    zoom.reset();
    plot?.focus({ preventScroll: true });
  }
</script>

<div class="zoombar">
  <span class="hint" aria-hidden="true">{hint ?? t("ui.zoom.hint")}</span>
  {#if zoom.view}
    <span class="range">{range}</span>
    <button type="button" class="clear" onclick={reset}>{t("ui.zoom.reset")}</button>
  {/if}
</div>

<style>
  .zoombar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 12px;
    min-height: 26px;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .hint {
    margin-right: auto;
    color: var(--ink3);
  }
  .clear {
    padding: 2px 6px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--accent);
    font: 600 12.5px/1.35 var(--f);
    cursor: pointer;
  }
</style>
