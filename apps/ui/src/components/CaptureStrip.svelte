<script lang="ts">
  import { captures } from "../lib/capture.svelte";
  import { formatTime } from "../lib/format";
  import { helperJobs } from "../lib/helper.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href, router } from "../lib/router.svelte";
  import { rowOf } from "../screens/capture/words";
  import StatusMark from "./StatusMark.svelte";

  // The strip above the screen title about the deep capture: it runs, a second one was refused, or
  // it finished and its result is not open yet. It sits below the monitor's banner.

  /** `m:ss`, or `h:mm:ss` from an hour. */
  const clock = (ms: number): string => {
    const s = Math.max(0, Math.round(ms / 1000));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = String(s % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
  };

  const run = $derived(captures.run);
  const done = $derived(captures.entries?.find((e) => e.summary.id === captures.finished) ?? null);
  const onCaptures = $derived(router.current.screen === "captures");
  const elapsed = $derived(run && run.started_ms !== null ? clock(captures.now - run.started_ms) : "");
  /** How far along a capture whose end is known is, from 0 to 100; `null` when the end is not known. */
  const progress = $derived.by((): number | null => {
    if (!run || run.started_ms === null || run.ends_ms === null) return null;
    const span = Math.max(1, run.ends_ms - run.started_ms);
    return Math.min(100, Math.max(0, ((captures.now - run.started_ms) / span) * 100));
  });
  const sentence = $derived.by((): string => {
    if (!run) return "";
    if (run.started_ms === null) return t("ui.captures.strip.earlier");
    if (run.ends_ms === null) return t("ui.captures.strip.since", { elapsed, time: formatTime(run.started_ms) });
    return t("ui.captures.strip.known", {
      elapsed,
      total: clock(run.ends_ms - run.started_ms),
      time: formatTime(run.ends_ms),
    });
  });
</script>

{#if run && captures.refused}
  <div class="strip" role="alert">
    <span class="icon"><StatusMark kind="attention" size={16} word="" /></span>
    <div class="text">
      <p><b>{t("ui.captures.strip.refused")}</b> {t("ui.captures.strip.refused.body")}</p>
    </div>
    <button type="button" class="btn" onclick={() => captures.dismiss()}>
      {t("ui.captures.strip.got_it")}
    </button>
  </div>
{:else if run}
  <div class="strip" role="status">
    <span class="icon"><StatusMark kind="info" size={16} word="" /></span>
    <div class="text">
      <p><b>{t("ui.captures.strip.title")}</b> {sentence}</p>
      {#if !run.own}
        <p class="note">{t("ui.captures.strip.who")}</p>
      {/if}
      {#if captures.cancelling}
        <p class="note">{t("ui.helper.cancelling.note")}</p>
      {/if}
      {#if progress !== null}
        <span class="pbar" aria-hidden="true"><i style:width="{progress}%"></i></span>
      {:else}
        <span class="pbar unknown" aria-hidden="true"></span>
      {/if}
    </div>
    {#if !onCaptures}
      <a class="btn" href={href.captures()}>{t("ui.captures.strip.see")}</a>
    {:else if run.own}
      {#if captures.cancelling}
        <button type="button" class="btn" disabled aria-disabled="true">{t("ui.helper.cancelling")}</button>
      {:else}
        <button type="button" class="btn" onclick={() => helperJobs.ask("capture")}>{t("ui.helper.cancel.capture")}</button>
      {/if}
    {/if}
  </div>
{:else if captures.finished !== null}
  <div class="strip" role="status">
    <span class="icon"><StatusMark kind="ok" size={16} word="" /></span>
    <div class="text">
      <p>
        <b>{t("ui.captures.strip.done")}</b>
        {#if done}{@const row = rowOf(done)}{row.title}. {row.sub}{/if}
      </p>
    </div>
    <a class="btn accent" href={href.capture(captures.finished)}>{t("ui.captures.finished.open")}</a>
    <button type="button" class="btn" onclick={() => captures.dismiss()}>{t("ui.captures.strip.close")}</button>
  </div>
{/if}

<style>
  .strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    padding: 10px 14px;
    background: color-mix(in srgb, var(--ink) 6%, var(--main));
    border: 1px solid var(--stroke);
    border-radius: 12px;
  }
  .icon {
    display: grid;
    flex: none;
  }
  .text {
    flex: 1 1 30ch;
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
    font-size: 12.5px;
  }
  .pbar {
    display: block;
    height: 4px;
    margin-top: 8px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--stroke);
  }
  .pbar i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--accent);
  }
  .pbar.unknown {
    background: repeating-linear-gradient(90deg, var(--stroke) 0 6px, transparent 6px 12px);
  }
  @media (forced-colors: active) {
    .strip {
      border-color: CanvasText;
    }
    .pbar i {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
