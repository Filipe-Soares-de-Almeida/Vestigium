<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { formatDuration, formatTime } from "../lib/format";
  import { keepFocus } from "../lib/focus";
  import { t } from "../lib/i18n.svelte";
  import { failingSince, sourceKey } from "../lib/monitor";
  import { router } from "../lib/router.svelte";
  import Icon from "./Icon.svelte";

  type Kind = "paused" | "stopped" | "unreachable" | "silent" | "no_reading" | "degraded" | "unverified";

  // One banner above the screen title; none while every source reads. A health check that has not
  // finished shows only on the Health screen.
  const kind = $derived.by((): Kind | null => {
    if (app.paused) return "paused";
    if (app.stopped) return "stopped";
    if (app.unreachable) return "unreachable";
    if (app.silent) return "silent";
    if (app.noReading) return "no_reading";
    if (app.degraded) return "degraded";
    if (app.unverified !== null && router.current.screen === "health") return "unverified";
    return null;
  });
  const neutral = $derived(kind === "silent" || kind === "no_reading" || kind === "degraded" || kind === "unverified");
  const technical = $derived(app.settings?.settings.detail_level === "Technical");

  /** The sentence under the title for the states the monitor reports about itself. */
  const body = $derived.by((): string => {
    const monitor = app.monitor;
    if (kind === "paused") {
      return app.pausedUntil !== null ? t("ui.shell.paused.body_until", { time: formatTime(app.pausedUntil) }) : t("ui.shell.paused.body");
    }
    if (kind === "silent") return t("ui.shell.silent.body", { seconds: app.silentSeconds });
    if (monitor === null) return kind === null ? "" : t(`ui.shell.${kind}.body`);
    if (kind === "no_reading") {
      const since = monitor.no_reading_since_ms ?? app.started;
      return t("ui.shell.no_reading.body", { time: formatTime(since), duration: formatDuration(Math.max(0, Date.now() - since)) });
    }
    if (kind === "degraded") {
      const since = failingSince(monitor.failing) ?? Date.now();
      const source =
        monitor.failing.length === 1
          ? t(sourceKey(monitor.failing[0].source))
          : t("ui.monitor.source.several", { count: monitor.failing.length });
      return t("ui.shell.degraded.body", { source, time: formatTime(since) });
    }
    if (kind === "unverified") {
      const ok = monitor.health_ok_ms;
      return ok === null
        ? t("ui.shell.unverified.body_never", { time: formatTime(app.started) })
        : t("ui.shell.unverified.body", { time: formatTime(ok) });
    }
    return t(`ui.shell.${kind}.body`);
  });

  /** What the system said about each source that fails, for the technical level. */
  const technicalLines = $derived.by((): string[] => {
    const monitor = app.monitor;
    if (!technical || monitor === null || (kind !== "no_reading" && kind !== "degraded")) return [];
    const lines = monitor.failing.map((f) => `${t(sourceKey(f.source))}: ${f.error}`);
    if (lines.length === 0 && kind === "no_reading" && app.lastError) lines.push(app.lastError);
    return lines;
  });

  /** Runs an action whose success removes the banner, then keeps focus inside the screen. */
  function act(action: () => Promise<void>): void {
    const main = document.getElementById("main");
    if (main) void keepFocus(main, action);
    else void action();
  }
</script>

{#if kind}
  <div class="banner" class:neutral>
    <span class="icon"><Icon name={kind} /></span>
    <div class="text">
      <p>
        <b>{t(`ui.shell.${kind}.title`)}</b>
        {body}
      </p>
      {#if kind === "stopped" && app.startError}
        <p class="note" role="status">{errorText(app.startError)}</p>
      {/if}
      {#each technicalLines as line (line)}
        <p class="note mono">{line}</p>
      {/each}
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
    {:else if kind !== "unverified"}
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
  .banner.neutral {
    background: color-mix(in srgb, var(--ink) 6%, var(--main));
    border-color: var(--stroke);
  }
  .icon {
    display: grid;
    flex: none;
    color: var(--press);
  }
  .neutral .icon {
    color: var(--ink2);
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
  .mono {
    font-family: var(--mono);
    overflow-wrap: anywhere;
  }
  @media (forced-colors: active) {
    .banner {
      border-color: CanvasText;
    }
  }
</style>
