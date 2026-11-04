<script lang="ts" module>
  /** How something stands in Health: shape and word together, so colour never carries it alone. */
  export type MarkKind = "Critical" | "Attention" | "Ok" | "Unknown" | "Closed" | "Partial" | "Running" | "Recommended";
</script>

<script lang="ts">
  import { t } from "../lib/i18n.svelte";

  let {
    mark,
    word = true,
    silent = false,
  }: {
    mark: MarkKind;
    word?: boolean;
    /** The text beside the mark already says the state, so a screen reader hears it once. */
    silent?: boolean;
  } = $props();
</script>

<span class="cn {mark}">
  <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true" focusable="false">
    {#if mark === "Critical"}
      <path d="M4.3 1h5.4L13 4.3v5.4L9.7 13H4.3L1 9.7V4.3z" fill="currentColor" />
      <path d="M7 3.8v3.6M7 9.6v.1" stroke="var(--card)" stroke-width="1.6" stroke-linecap="round" />
    {:else if mark === "Attention"}
      <path d="M7 1.6l5.6 10H1.4z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
      <path d="M7 5.6v2.6M7 10v.1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
    {:else if mark === "Ok"}
      <circle cx="7" cy="7" r="5.8" fill="none" stroke="currentColor" stroke-width="1.4" />
      <path
        d="M4.6 7.1l1.6 1.6 3.2-3.3"
        fill="none"
        stroke="currentColor"
        stroke-width="1.4"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    {:else if mark === "Closed"}
      <path
        d="M3 7.4l2.6 2.6L11 4.6"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    {:else if mark === "Partial"}
      <circle cx="7" cy="7" r="5.8" fill="none" stroke="currentColor" stroke-width="1.4" />
      <path d="M7 1.2a5.8 5.8 0 010 11.6z" fill="currentColor" />
    {:else if mark === "Recommended"}
      <circle cx="7" cy="7" r="5.8" fill="none" stroke="currentColor" stroke-width="1.4" />
      <circle cx="7" cy="7" r="2.6" fill="currentColor" />
    {:else if mark === "Running"}
      <path class="spin" d="M7 1.4a5.6 5.6 0 105.6 5.6" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    {:else}
      <circle cx="7" cy="7" r="5.8" fill="none" stroke="currentColor" stroke-width="1.4" stroke-dasharray="2.2 2" />
    {/if}
  </svg>
  {#if word}{t(`ui.health.mark.${mark}`)}{:else if !silent}<span class="sr">{t(`ui.health.mark.${mark}`)}</span>{/if}
</span>

<style>
  .cn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ink2);
    font: 600 13px/1.35 var(--f);
    white-space: nowrap;
  }
  svg {
    flex: none;
    color: var(--ink3);
  }
  .Critical svg {
    color: var(--freeze);
  }
  .Attention svg {
    color: var(--press);
  }
  .Partial svg {
    color: var(--ink2);
  }
  .Running svg,
  .Recommended svg {
    color: var(--accent);
  }
  .spin {
    transform-origin: 7px 7px;
    animation: spin 1.1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  @media (forced-colors: active) {
    svg {
      color: CanvasText;
    }
  }
</style>
