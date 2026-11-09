<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import type { Zoom } from "../lib/zoom.svelte";

  let {
    zoom,
    range,
    plot,
  }: {
    zoom: Zoom;
    /** The stretch on screen, worded for the reader; announced while zoomed. */
    range: string;
    /** The chart's plot, which gets focus back when the reset button turns off under it. */
    plot?: HTMLElement;
    /** Kept for callers that word their gestures; the buttons speak for themselves. */
    hint?: string;
  } = $props();

  function reset(): void {
    zoom.reset();
    plot?.focus({ preventScroll: true });
  }
</script>

<div class="zoombar" role="group" aria-label={t("ui.zoom.label")}>
  <button type="button" aria-label={t("ui.zoom.out")} title={t("ui.zoom.out")} disabled={!zoom.view} onclick={() => zoom.center(1.4)}>−</button>
  <button type="button" aria-label={t("ui.zoom.in")} title={t("ui.zoom.in")} onclick={() => zoom.center(0.7)}>+</button>
  <button type="button" disabled={!zoom.view} onclick={reset}>{t("ui.zoom.reset")}</button>
  <span class="sr" aria-live="polite">{zoom.view ? t("ui.zoom.said", { range }) : ""}</span>
</div>

<style>
  .zoombar {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  button {
    height: 26px;
    min-width: 26px;
    padding: 0 8px;
    border: 1px solid var(--stroke);
    border-radius: 7px;
    background: var(--ctl);
    color: var(--ink2);
    font: 600 13px/1 var(--f);
    cursor: pointer;
  }
  button:hover:enabled {
    background: var(--card2);
    color: var(--ink);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
