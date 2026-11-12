<script lang="ts">
  import type { ShellTone } from "../lib/status";

  let { tone }: { tone: ShellTone } = $props();
</script>

{#if tone === "no_reading" || tone === "partial" || tone === "silent" || tone === "unverified"}
  <svg class="mark" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" focusable="false">
    <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.4" />
    {#if tone === "no_reading"}
      <path d="M5.2 8h5.6" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    {:else if tone === "partial"}
      <path d="M8 4.6v4M8 11.2v.1" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    {:else if tone === "silent"}
      <path d="M4.4 11.6l7.2-7.2" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    {:else}
      <circle cx="8" cy="8" r="1.5" fill="currentColor" />
    {/if}
  </svg>
{:else}
  <i class={tone} aria-hidden="true"></i>
{/if}

<style>
  i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  i.ok {
    background: var(--good);
  }
  i.paused,
  i.off {
    box-shadow: inset 0 0 0 2px var(--ink3);
  }
  .mark {
    flex: none;
    color: var(--ink2);
  }
  @media (forced-colors: active) {
    i.ok {
      forced-color-adjust: none;
      background: CanvasText;
    }
    i.paused,
    i.off {
      forced-color-adjust: none;
      box-shadow: inset 0 0 0 2px CanvasText;
    }
    .mark {
      color: CanvasText;
    }
  }
</style>
