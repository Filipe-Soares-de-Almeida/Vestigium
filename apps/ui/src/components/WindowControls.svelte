<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";

  import { t } from "../lib/i18n.svelte";

  const win = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    const sync = async () => {
      maximized = await win.isMaximized();
      document.documentElement.toggleAttribute("data-maximized", maximized);
    };
    void sync();
    const off = win.onResized(sync);
    return () => void off.then((unlisten) => unlisten());
  });

  const resize = $derived(maximized ? t("ui.shell.window.restore") : t("ui.shell.window.maximize"));
</script>

<!-- Minimize, maximize or restore, and close, drawn as the Windows 11 caption buttons. -->
<div class="captions" role="group" aria-label={t("ui.shell.window.label")}>
  <button
    type="button"
    class="caption"
    aria-label={t("ui.shell.window.minimize")}
    title={t("ui.shell.window.minimize")}
    onclick={() => win.minimize()}
  >
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" focusable="false">
      <path d="M0 5.5h10" />
    </svg>
  </button>
  <button type="button" class="caption" aria-label={resize} title={resize} onclick={() => win.toggleMaximize()}>
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" focusable="false">
      {#if maximized}
        <rect x="0.5" y="2.5" width="7" height="7" rx="1.5" />
        <path d="M2.5 2.5V2A1.5 1.5 0 0 1 4 .5h4A1.5 1.5 0 0 1 9.5 2v4A1.5 1.5 0 0 1 8 7.5h-.5" />
      {:else}
        <rect x="0.5" y="0.5" width="9" height="9" rx="1.5" />
      {/if}
    </svg>
  </button>
  <button
    type="button"
    class="caption close"
    aria-label={t("ui.shell.window.close")}
    title={t("ui.shell.window.close")}
    onclick={() => win.close()}
  >
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true" focusable="false">
      <path d="M.5.5l9 9M9.5.5l-9 9" />
    </svg>
  </button>
</div>

<style>
  .captions {
    position: absolute;
    top: 0;
    right: 0;
    z-index: 30;
    display: flex;
    height: 40px;
  }
  .caption {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--ink2);
  }
  .caption:focus-visible {
    outline-offset: -2px;
  }
  .caption svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1;
  }
  .caption:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .caption:active {
    background: var(--row-press);
  }
  .close:hover,
  .close:active {
    background: var(--caption-close);
    color: var(--on-caption-close);
  }
  .close:active {
    opacity: 0.9;
  }
  :global(:root[data-inactive]) .caption:not(:hover) {
    color: var(--ink3);
  }
  @media (forced-colors: active) {
    .caption:hover,
    .close:hover {
      forced-color-adjust: none;
      background: Highlight;
      color: HighlightText;
    }
  }
</style>
