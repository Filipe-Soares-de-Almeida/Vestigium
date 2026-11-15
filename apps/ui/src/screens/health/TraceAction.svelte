<script lang="ts">
  import AdminPill from "../../components/AdminPill.svelte";
  import HelperDialogs from "../../components/HelperDialogs.svelte";
  import HelperLine from "../../components/HelperLine.svelte";
  import Icon from "../../components/Icon.svelte";
  import { bootTrace } from "../../lib/bootTrace.svelte";
  import { t } from "../../lib/i18n.svelte";

  /** What the trace of the latest startup is for, and the button that copies it with administrator permission. */
  const running = $derived(bootTrace.phase === "running");
</script>

<p class="p">{t("ui.health.startup.trace.body")}</p>
<HelperLine job="trace" />
{#if bootTrace.phase === "done"}
  <p class="p" role="status">
    <b>{t("ui.health.startup.trace.done")}</b>
    <span class="file">{bootTrace.fileName}</span>
  </p>
{:else if bootTrace.phase === "missing"}
  <p class="p" role="status"><b>{t("ui.health.startup.trace.missing.title")}</b> {t("ui.health.startup.trace.missing.body")}</p>
{:else if bootTrace.phase === "failed"}
  <p class="p" role="status"><b>{t("ui.health.startup.trace.failed.title")}</b> {t("ui.health.startup.trace.failed.body")}</p>
{/if}
<div class="acts">
  <button
    type="button"
    class="btn sm"
    class:accent={bootTrace.phase !== "done"}
    disabled={running}
    aria-busy={running}
    onclick={() => void bootTrace.copy()}
  >
    {#if running}
      <span class="spin" aria-hidden="true"></span>{t("ui.health.startup.trace.running")}
    {:else}
      <Icon name="lock" />{t(bootTrace.phase === "done" ? "ui.health.startup.trace.again" : "ui.health.startup.trace.copy")}
    {/if}
  </button>
  <AdminPill />
  {#if bootTrace.phase === "done"}
    <button type="button" class="btn sm" onclick={() => void bootTrace.openFolder()}>
      <Icon name="outside" />{t("ui.health.startup.trace.open")}
    </button>
  {/if}
</div>
<HelperDialogs job="trace" onretry={() => void bootTrace.copy()} />

<style>
  .p b {
    font-weight: 600;
  }
  .file {
    overflow-wrap: anywhere;
  }
  .acts {
    margin-top: 8px;
  }
  .btn.sm {
    min-height: 30px;
    padding: 0 14px;
    font-size: 13.5px;
  }
  .btn :global(svg) {
    flex: none;
    width: 16px;
    height: 16px;
  }
  .spin {
    flex: none;
    width: 14px;
    height: 14px;
    border: 2px solid color-mix(in srgb, currentColor 30%, transparent);
    border-top-color: currentColor;
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
      border-top-color: ButtonText;
    }
  }
</style>
