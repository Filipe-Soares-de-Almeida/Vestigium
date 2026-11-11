<script lang="ts">
  import { captures } from "../../lib/capture.svelte";
  import { focusHere } from "../../lib/focus";
  import { helperJobs } from "../../lib/helper.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { CaptureRun } from "../../lib/types";

  let { run }: { run: CaptureRun } = $props();

  const left = $derived(Math.max(0, run.ends_ms - captures.now));
  const clock = (ms: number) => {
    const s = Math.max(0, Math.round(ms / 1000));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = String(s % 60).padStart(2, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
  };
  const done = $derived(
    Math.min(100, Math.max(0, ((captures.now - run.started_ms) / Math.max(1, run.ends_ms - run.started_ms)) * 100)),
  );
</script>

<section class="wait" aria-labelledby="wait-title" role="status">
  <p class="state">
    <StatusMark kind="overload" word={t("ui.captures.wait.mark")} />
    <span id="wait-title" tabindex="-1" use:focusHere>{t("ui.captures.wait.keep")}</span>
  </p>
  <p class="clock">
    <b>{clock(captures.now - run.started_ms)}</b>
    {t("ui.captures.wait.of", { total: clock(run.ends_ms - run.started_ms) })}
    {#if left === 0}· {t("ui.captures.wait.finishing")}{/if}
  </p>
  <span class="pbar" aria-hidden="true"><i style:width="{done}%"></i></span>
  <p class="note">{t("ui.captures.wait.body", { time: formatTime(run.ends_ms) })}</p>
  {#if captures.cancelling}
    <p class="note" role="status">{t("ui.helper.cancelling.note")}</p>
  {/if}
  <div class="acts">
    {#if captures.cancelling}
      <button type="button" class="btn" disabled aria-disabled="true">{t("ui.helper.cancelling")}</button>
    {:else}
      <button type="button" class="btn" onclick={() => helperJobs.ask("capture")}>{t("ui.helper.cancel.capture")}</button>
    {/if}
  </div>
</section>

<style>
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .state span {
    outline: none;
  }
  .clock {
    margin: 6px 0 10px;
    color: var(--ink2);
    font-size: 14px;
  }
  .clock b {
    margin-right: 6px;
    color: var(--ink);
    font: 350 28px/1.1 var(--fd);
  }
  .pbar {
    display: block;
    max-width: 420px;
    height: 4px;
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
  @media (forced-colors: active) {
    .pbar i {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
