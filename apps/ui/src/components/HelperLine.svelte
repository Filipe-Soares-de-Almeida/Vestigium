<script lang="ts">
  import { helperJobs, type JobKey } from "../lib/helper.svelte";
  import { t } from "../lib/i18n.svelte";
  import StatusMark from "./StatusMark.svelte";

  // What became of the last attempt of a helper job that left no result: cancelled, or the helper
  // did not finish in time. It stays on the job's card until the next attempt starts.
  let { job, volume }: { job: JobKey; volume?: string } = $props();

  const attempt = $derived(helperJobs.last[job]);
  const shown = $derived(attempt !== undefined && (volume === undefined || attempt.volume === volume));
</script>

{#if shown && attempt}
  <p class="line" role="status">
    {#if attempt.kind === "timeout"}
      <StatusMark kind="attention" size={14} word={t("ui.helper.timeout.title")} />
      <span>{t("ui.helper.timeout.last")}</span>
    {:else}
      <StatusMark kind="info" size={14} word={t(`ui.helper.cancelled.${job}.title`)} />
      <span><b>{t(`ui.helper.cancelled.${job}.title`)}</b> {t(`ui.helper.cancelled.${job}.body`)}</span>
    {/if}
  </p>
{/if}

<style>
  .line {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    max-width: 72ch;
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.55;
  }
  .line :global(.status) {
    margin-top: 3px;
  }
  .line :global(.word) {
    display: none;
  }
  b {
    font-weight: 600;
  }
</style>
