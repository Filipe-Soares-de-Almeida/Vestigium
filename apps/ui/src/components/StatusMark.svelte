<script lang="ts" module>
  export type StatusKind = "critical" | "overload" | "attention" | "ok" | "info";
</script>

<script lang="ts">
  import { t } from "../lib/i18n.svelte";

  let {
    kind,
    size = 16,
    reveal = false,
    word,
  }: {
    kind: StatusKind;
    size?: number;
    /** In a list row the word stays hidden until the row is hovered or focused. */
    reveal?: boolean;
    /** Replaces the default word for the kind. */
    word?: string;
  } = $props();

  const text = $derived(word ?? t(`ui.status.${kind}`));
</script>

<span class="status {kind}" class:reveal title={text}>
  <svg width={size} height={size} viewBox="0 0 16 16" aria-hidden="true" focusable="false">
    {#if kind === "critical"}
      <circle cx="8" cy="8" r="7.2" fill="currentColor" />
      <path class="cut" d="M5.5 5.5l5 5M10.5 5.5l-5 5" />
    {:else if kind === "overload"}
      <circle cx="8" cy="8" r="7.2" fill="currentColor" />
      <path class="cut" d="M8 4.4v4.3M8 11.3v.1" />
    {:else if kind === "attention"}
      <path d="M8 1.7l6.6 11.5H1.4z" fill="currentColor" stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" />
      <path class="cut" d="M8 6v3.3M8 11.2v.1" />
    {:else if kind === "ok"}
      <circle cx="8" cy="8" r="7.2" fill="currentColor" />
      <path class="cut" d="M4.9 8.2l2.2 2.2 4-4.4" />
    {:else}
      <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.4" />
      <path d="M8 7.2v3.5M8 5v.1" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    {/if}
  </svg>
  <span class="word">{text}</span>
</span>

<style>
  .status {
    display: inline-flex;
    align-items: center;
    color: var(--ink2);
    font: 400 13px/1.35 var(--f);
    white-space: nowrap;
  }
  svg {
    flex: none;
  }
  .word {
    padding-left: 7px;
  }
  .critical svg {
    color: var(--freeze);
  }
  .overload svg {
    color: var(--press);
  }
  .attention svg {
    color: var(--busy);
  }
  .ok svg {
    color: var(--good);
  }
  .cut {
    fill: none;
    stroke: var(--layer);
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .reveal .word {
    max-width: 0;
    padding-left: 0;
    overflow: hidden;
    opacity: 0;
    transition:
      max-width 0.44s cubic-bezier(0.2, 0.8, 0.2, 1),
      padding-left 0.44s cubic-bezier(0.2, 0.8, 0.2, 1),
      opacity 0.34s;
  }
  :global([data-reveal]:is(:hover, :focus-visible, :focus-within)) .reveal .word {
    max-width: 140px;
    padding-left: 7px;
    opacity: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    .reveal .word {
      transition: none;
    }
  }
  @media (forced-colors: active) {
    svg {
      color: CanvasText;
    }
    .cut {
      stroke: Canvas;
    }
  }
</style>
