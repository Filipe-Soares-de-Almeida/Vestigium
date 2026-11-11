<script lang="ts">
  import { tick } from "svelte";

  import { helperJobs, type JobKey } from "../lib/helper.svelte";
  import { t } from "../lib/i18n.svelte";

  // The two questions a helper job's screen can put: whether to cancel a long job, and, once, that
  // the helper did not finish in time. Both wait for the person on the job's own screen.
  let {
    job,
    onconfirm,
    onretry,
  }: {
    job: JobKey;
    /** Cancels the job; only the long jobs ask first. */
    onconfirm?: () => void;
    /** Starts the job again; a job that cannot time out has none. */
    onretry?: () => void;
  } = $props();

  let dialog = $state<HTMLDialogElement>();
  let safe = $state<HTMLButtonElement>();

  const asking = $derived(onconfirm !== undefined && helperJobs.asking === job);
  const late = $derived(!asking && onretry !== undefined && helperJobs.unseen(job));
  const open = $derived(asking || late);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      dialog.showModal();
      void tick().then(() => safe?.focus());
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function keep(): void {
    if (asking) helperJobs.dismiss();
    else helperJobs.see(job);
  }

  function confirm(): void {
    helperJobs.dismiss();
    onconfirm?.();
  }

  function retry(): void {
    helperJobs.see(job);
    onretry?.();
  }
</script>

<dialog
  class="helper"
  bind:this={dialog}
  aria-labelledby="helper-title"
  aria-describedby="helper-body"
  oncancel={(event) => {
    event.preventDefault();
    keep();
  }}
>
  {#if open}
    <div class="in">
      <span class="tag">{t(`ui.helper.name.${job}`)}</span>
      {#if asking}
        <h2 id="helper-title">{t(`ui.helper.ask.${job}.title`)}</h2>
        <p id="helper-body">{t(`ui.helper.ask.${job}.body`)}</p>
      {:else}
        <h2 id="helper-title">{t("ui.helper.timeout.title")}</h2>
        <p id="helper-body">{t(`ui.helper.timeout.body.${job}`)}</p>
      {/if}
    </div>
    <div class="acts">
      <button type="button" class="btn" bind:this={safe} onclick={keep}>
        {asking ? t(`ui.helper.ask.${job}.no`) : t("ui.helper.timeout.close")}
      </button>
      {#if asking}
        <button type="button" class="btn accent" onclick={confirm}>{t(`ui.helper.ask.${job}.yes`)}</button>
      {:else}
        <button type="button" class="btn accent" onclick={retry}>{t("ui.common.retry")}</button>
      {/if}
    </div>
  {/if}
</dialog>

<style>
  .helper {
    width: min(460px, calc(100vw - 48px));
    padding: 0;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .helper::backdrop {
    background: #0009;
  }
  .in {
    display: grid;
    gap: 10px;
    justify-items: start;
    padding: 20px 22px 16px;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--ink) 12%, transparent);
    font-size: 12px;
  }
  h2 {
    margin: 0;
    font: 600 17px/1.35 var(--f);
  }
  p {
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.55;
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
    margin: 0;
    padding: 14px 22px;
    border-top: 1px solid var(--stroke2);
    border-radius: 0 0 12px 12px;
    background: var(--card2);
  }
  @media (forced-colors: active) {
    .helper {
      border-color: CanvasText;
    }
  }
</style>
