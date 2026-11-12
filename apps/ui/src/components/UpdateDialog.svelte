<script lang="ts">
  import { tick } from "svelte";

  import { captures } from "../lib/capture.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import { updates } from "../lib/update.svelte";
  import { formatSize } from "../screens/settings/data";
  import ReleaseNotes from "./ReleaseNotes.svelte";

  // The update's dialogs: the first-open question, a newer version, and the notes after updating.
  let dialog = $state<HTMLDialogElement>();
  let first = $state<HTMLButtonElement>();

  const kind = $derived(updates.dialog);
  const found = $derived(updates.found);
  const busy = $derived(hwtests.running !== null || captures.run !== null);
  const progress = $derived.by(() => {
    const d = updates.download;
    if (!d || !found) return null;
    const share = d.total ? Math.min(1, d.done / d.total) : 0;
    const text = d.total
      ? t("ui.update.progress", { version: found.version, done: formatSize(d.done), total: formatSize(d.total) })
      : t("ui.update.progress_unknown", { version: found.version, done: formatSize(d.done) });
    return { share, text };
  });

  $effect(() => {
    if (!dialog) return;
    if (kind && !dialog.open) {
      dialog.showModal();
      void tick().then(() => first?.focus());
    } else if (!kind && dialog.open) {
      dialog.close();
    }
  });
</script>

<dialog
  class="update"
  bind:this={dialog}
  aria-labelledby="update-title"
  oncancel={(event) => {
    event.preventDefault();
    updates.close();
  }}
>
  {#if kind === "ask"}
    <div class="in">
      <h2 id="update-title">{t("ui.update.ask.title")}</h2>
      <p>{t("ui.update.ask.body")}</p>
      <p>{t("ui.update.ask.later")}</p>
    </div>
    <div class="acts">
      <button type="button" class="btn" onclick={() => updates.answer(false)}>{t("ui.update.ask.no")}</button>
      <button type="button" class="btn accent" bind:this={first} onclick={() => updates.answer(true)}>
        {t("ui.update.ask.yes")}
      </button>
    </div>
  {:else if kind === "available" && found}
    <div class="in">
      <h2 id="update-title">{t("ui.update.available.title", { version: found.version })}</h2>
      <p>{t("ui.update.available.body", { current: updates.version })}</p>
      <ReleaseNotes summary={found.summary} groups={found.groups} />
      {#if progress}
        <div class="progress" role="status">
          <div
            class="bar"
            role="progressbar"
            aria-label={progress.text}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={Math.round(progress.share * 100)}
          >
            <i style:transform="translateX({(progress.share - 1) * 100}%)"></i>
          </div>
          <p>{updates.installing ? t("ui.update.installing") : progress.text}</p>
        </div>
      {/if}
      {#if updates.failed}<p class="fail" role="alert">{t("ui.update.failed")}</p>{/if}
      {#if busy && !progress}<p class="note">{t("ui.update.available.busy")}</p>{/if}
    </div>
    <div class="acts">
      {#if progress}
        <button type="button" class="btn" disabled={updates.installing} bind:this={first} onclick={() => updates.cancel()}>
          {t("ui.update.cancel")}
        </button>
      {:else}
        <button type="button" class="btn" onclick={() => updates.close()}>{t("ui.update.available.later")}</button>
        <button type="button" class="btn accent" bind:this={first} disabled={busy} onclick={() => updates.install()}>
          {t("ui.update.available.install")}
        </button>
      {/if}
    </div>
  {:else if kind === "news"}
    <div class="in">
      <h2 id="update-title">{t("ui.update.news.title", { version: updates.version })}</h2>
      {#if updates.news.length > 1}<p>{tn("ui.update.news.since", updates.news.length)}</p>{/if}
      <ReleaseNotes releases={updates.news} />
    </div>
    <div class="acts">
      <button type="button" class="btn accent" bind:this={first} onclick={() => updates.close()}>{t("ui.update.news.ok")}</button>
    </div>
  {/if}
</dialog>

<style>
  .update {
    width: min(480px, calc(100vw - 48px));
    max-height: min(620px, calc(100vh - 80px));
    padding: 0;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .update[open] {
    display: flex;
    flex-direction: column;
  }
  .update::backdrop {
    background: #0007;
  }
  .in {
    display: grid;
    gap: 10px;
    padding: 20px 22px 16px;
    overflow: auto;
  }
  h2 {
    margin: 0;
    font: 600 20px/1.3 var(--fd);
  }
  p {
    margin: 0;
    color: var(--ink2);
    line-height: 1.5;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
    padding: 14px 22px;
    border-top: 1px solid var(--stroke2);
    border-radius: 0 0 12px 12px;
    background: var(--card2);
  }
  .progress {
    display: grid;
    gap: 6px;
    margin-top: 4px;
  }
  .bar {
    height: 6px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 999px;
    background: var(--card2);
  }
  .bar i {
    display: block;
    width: 100%;
    height: 100%;
    background: var(--accent);
    transition: transform 0.25s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
  .progress p,
  .note {
    font-size: 13px;
  }
  .fail {
    color: var(--press);
    font-size: 13.5px;
  }
  @media (prefers-reduced-motion: reduce) {
    .bar i {
      transition: none;
    }
  }
  @media (forced-colors: active) {
    .update {
      border-color: CanvasText;
    }
    .bar i {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
