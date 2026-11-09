<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { formatTime } from "../lib/format";
  import { keepFocus } from "../lib/focus";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  // One banner above the screen title: paused, stopped or unreachable; none while watching.
  const kind = $derived(
    app.paused ? "paused" : app.stopped ? "stopped" : app.unreachable ? "unreachable" : null,
  );

  /** Runs an action whose success removes the banner, then keeps focus inside the screen. */
  function act(action: () => Promise<void>): void {
    const main = document.getElementById("main");
    if (main) void keepFocus(main, action);
    else void action();
  }
</script>

{#if kind}
  <div class="banner">
    <span class="icon"><Icon name={kind} /></span>
    <div class="text">
      <p>
        <b>{t(`ui.shell.${kind}.title`)}</b>
        {kind === "paused" && app.pausedUntil !== null
          ? t("ui.shell.paused.body_until", { time: formatTime(app.pausedUntil) })
          : t(`ui.shell.${kind}.body`)}
      </p>
      {#if kind === "stopped" && app.startError}
        <p class="note" role="status">{errorText(app.startError)}</p>
      {/if}
    </div>
    {#if kind === "paused"}
      <button type="button" class="btn accent" data-focus="banner" onclick={() => act(() => app.resume())}>
        {t("ui.shell.paused.resume")}
      </button>
    {:else if kind === "stopped"}
      <button
        type="button"
        class="btn accent"
        data-focus="banner"
        aria-disabled={app.starting}
        onclick={() => !app.starting && act(() => app.startWatcher())}
      >
        {app.starting ? t("ui.shell.stopped.starting") : t("ui.shell.stopped.start")}
      </button>
    {:else}
      <button type="button" class="btn" data-focus="banner" onclick={() => act(() => app.refresh())}>
        {t("ui.common.retry")}
      </button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding: 10px 14px;
    background: var(--press-t);
    border: 1px solid var(--stroke2);
    border-radius: 12px;
  }
  .icon {
    display: grid;
    flex: none;
    color: var(--press);
  }
  .text {
    flex: 1 1 24ch;
    min-width: 0;
  }
  p {
    margin: 0;
  }
  b {
    font-weight: 600;
  }
  .note {
    margin-top: 2px;
    color: var(--ink2);
    font-size: 13px;
  }
  @media (forced-colors: active) {
    .banner {
      border-color: CanvasText;
    }
  }
</style>
