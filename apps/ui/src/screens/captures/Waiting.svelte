<script lang="ts">
  import { captures } from "../../lib/capture.svelte";
  import { focusHere } from "../../lib/focus";
  import { formatDuration, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { CaptureRun } from "../../lib/types";

  let { run }: { run: CaptureRun } = $props();

  const left = $derived(Math.max(0, run.ends_ms - captures.now));
  const done = $derived(
    Math.min(100, Math.max(0, ((captures.now - run.started_ms) / Math.max(1, run.ends_ms - run.started_ms)) * 100)),
  );
</script>

<section class="card wait" aria-labelledby="wait-title">
  <div class="head">
    <h2 id="wait-title" tabindex="-1" use:focusHere>{t("ui.captures.wait.title")}</h2>
    <span class="left">{left > 0 ? t("ui.captures.wait.left", { time: formatDuration(left) }) : t("ui.captures.wait.finishing")}</span>
  </div>
  <div class="bar" aria-hidden="true"><i style:width="{done}%"></i></div>
  <p>{t("ui.captures.wait.body", { time: formatTime(run.ends_ms) })}</p>
</section>

<style>
  .wait {
    display: grid;
    gap: 10px;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 4px 12px;
  }
  h2 {
    margin: 0;
    outline: none;
  }
  .left {
    color: var(--ink2);
    font-weight: 600;
  }
  .bar {
    height: 8px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 999px;
    background: var(--card2);
  }
  .bar i {
    display: block;
    min-width: 6px;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
  }
  p {
    margin: 0;
    color: var(--ink2);
    line-height: 1.5;
    max-width: 62ch;
  }
  @media (forced-colors: active) {
    .bar i {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
