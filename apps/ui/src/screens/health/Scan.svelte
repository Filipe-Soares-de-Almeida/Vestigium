<script lang="ts">
  import AdminPill from "../../components/AdminPill.svelte";
  import Block from "../../components/Block.svelte";
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

<Block title={t("ui.health.scan.title")}>
  {#if health.phase === "running"}
    <p class="p status" role="status"><span class="spin" aria-hidden="true"></span>{t("ui.health.scan.running")}</p>
  {:else}
    {#if health.phase === "declined"}
      <p class="p" role="status"><b>{t("ui.health.scan.declined.title")}</b> {t("ui.health.scan.declined.body")}</p>
    {:else if health.phase === "failed"}
      <p class="p" role="status"><b>{t("ui.health.scan.failed.title")}</b> {t("ui.health.scan.failed.body")}</p>
    {:else if health.phase === "done"}
      <p class="p" role="status">
        <b>{t("ui.health.scan.done")}</b>
        {health.found ? tn("ui.health.scan.done.found", health.found) : t("ui.health.scan.done.none")}
      </p>
    {/if}
    <p class="p">{t(level === "Technical" ? "ui.health.scan.lede.tech" : "ui.health.scan.lede")}</p>
    <p class="last">
      {blocked
        ? t("ui.health.scan.blocked")
        : last === null
          ? t("ui.health.scan.never")
          : t("ui.health.scan.last", { when: when(last) })}
    </p>
    <div class="acts">
      <button
        type="button"
        class="btn"
        class:accent={health.phase !== "done"}
        disabled={blocked}
        onclick={() => void health.scan()}
      >
        <Icon name="lock" />
        {health.phase === "declined" || health.phase === "failed"
          ? t("ui.health.scan.again")
          : t("ui.health.scan.start")}
      </button>
      <AdminPill />
    </div>
  {/if}
</Block>

<style>
  .p b {
    font-weight: 600;
  }
  .last {
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .status {
    display: flex;
    gap: 10px;
    align-items: center;
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
