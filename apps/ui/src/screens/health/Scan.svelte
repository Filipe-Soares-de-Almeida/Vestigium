<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { app } from "../../lib/app.svelte";
  import { health } from "../../lib/health.svelte";
  import { t, tn } from "../../lib/i18n.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { when } from "./words";

  let { level }: { level: DetailLevel } = $props();

  const last = $derived(health.dto?.last_scan_ms ?? null);
  // The helper's result reaches the history through the watcher.
  const blocked = $derived(!app.running);
</script>

<section class="card scan" aria-labelledby="scan-title">
  <span class="icochip"><Icon name="health" /></span>
  <div class="body">
    <h2 id="scan-title">{t("ui.health.scan.title")}</h2>
    {#if health.phase === "running"}
      <p class="status" role="status"><span class="spin" aria-hidden="true"></span>{t("ui.health.scan.running")}</p>
    {:else}
      {#if health.phase === "declined"}
        <p role="status"><b>{t("ui.health.scan.declined.title")}</b> {t("ui.health.scan.declined.body")}</p>
      {:else if health.phase === "failed"}
        <p role="status"><b>{t("ui.health.scan.failed.title")}</b> {t("ui.health.scan.failed.body")}</p>
      {:else if health.phase === "done"}
        <p role="status">
          <b>{t("ui.health.scan.done")}</b>
          {health.found ? tn("ui.health.scan.done.found", health.found) : t("ui.health.scan.done.none")}
        </p>
      {/if}
      <p>{t(level === "Technical" ? "ui.health.scan.lede.tech" : "ui.health.scan.lede")}</p>
      <div class="line">
        <button
          type="button"
          class="btn"
          class:accent={health.phase !== "done"}
          disabled={blocked}
          onclick={() => void health.scan()}
        >
          <Icon name="shield" />
          {health.phase === "declined" || health.phase === "failed"
            ? t("ui.health.scan.again")
            : t("ui.health.scan.start")}
        </button>
        <span>
          {blocked
            ? t("ui.health.scan.blocked")
            : last === null
              ? t("ui.health.scan.never")
              : t("ui.health.scan.last", { when: when(last) })}
        </span>
      </div>
    {/if}
  </div>
</section>

<style>
  .scan {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 14px;
    align-items: start;
    margin-top: 14px;
  }
  .icochip {
    background: var(--card2);
    color: var(--ink2);
  }
  .body {
    display: grid;
    gap: 10px;
  }
  h2 {
    margin: 8px 0 0;
    font: 600 15px/1.3 var(--f);
  }
  p {
    max-width: 75ch;
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.45;
  }
  p b {
    color: var(--ink);
    font-weight: 600;
  }
  .status {
    display: flex;
    gap: 10px;
    align-items: center;
    color: var(--ink);
  }
  .spin {
    flex: none;
    width: 14px;
    height: 14px;
    border: 2px solid var(--stroke);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation-duration: 3s;
    }
  }
  @media (forced-colors: active) {
    .spin {
      forced-color-adjust: none;
      border-color: GrayText;
      border-top-color: Highlight;
    }
  }
</style>
