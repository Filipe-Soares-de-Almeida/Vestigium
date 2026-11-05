<script lang="ts">
  import ReleaseNotes from "../../components/ReleaseNotes.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { type Release, updates } from "../../lib/update.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";

  let notes = $state<Release[] | null>(null);
  let showing = $state(false);

  const status = $derived.by((): { text: string; error: boolean } | null => {
    if (updates.checking) return { text: t("ui.settings.updates.checking"), error: false };
    if (updates.result === "up_to_date") return { text: t("ui.settings.updates.up_to_date"), error: false };
    if (updates.result === "available" && updates.found)
      return { text: t("ui.update.available.title", { version: updates.found.version }), error: false };
    if (updates.result === "failed") return { text: t("ui.settings.updates.failed"), error: true };
    return null;
  });

  /** Asks the feed now; a newer version opens its dialog, as on opening the window. */
  async function checkNow(): Promise<void> {
    await updates.check();
    if (updates.result === "available") updates.dialog = "available";
  }

  async function toggleNotes(): Promise<void> {
    showing = !showing;
    if (showing && notes === null) notes = await updates.allNotes();
  }
</script>

<Row id="upd-auto" title={t("ui.settings.updates.auto")} hint={t("ui.settings.updates.auto_hint")}>
  <Toggle id="upd-auto" checked={updates.prefs?.check === true} onchange={(on) => updates.setCheck(on)} />
</Row>
<Row id="upd-version" title={t("ui.settings.updates.version")} hint={t("ui.settings.updates.version_hint", { version: updates.version })}>
  <button type="button" class="btn" disabled={updates.checking} onclick={checkNow}>
    {updates.result === "failed" ? t("ui.settings.updates.retry") : t("ui.settings.updates.check")}
  </button>
</Row>
{#if status}
  <p class="status" class:error={status.error} role="status">{status.text}</p>
{/if}
<Row id="upd-news" title={t("ui.settings.updates.news")} hint={t("ui.settings.updates.news_hint")}>
  <button type="button" class="btn" aria-expanded={showing} onclick={toggleNotes}>
    {showing ? t("ui.settings.updates.news_hide") : t("ui.settings.updates.news_show")}
  </button>
</Row>
{#if showing && notes !== null}
  <div class="notes">
    {#if notes.length}
      <ReleaseNotes releases={notes} />
    {:else}
      <p>{t("ui.settings.updates.news_none")}</p>
    {/if}
  </div>
{/if}

<style>
  .status {
    margin: 0 0 4px;
    padding: 0 18px;
    color: var(--ink2);
    font-size: 13px;
  }
  .status.error {
    color: var(--press);
  }
  .notes {
    padding: 14px 18px;
    border: 1px solid var(--stroke2);
    border-radius: 8px;
    background: var(--card);
  }
  .notes p {
    margin: 0;
    color: var(--ink2);
  }
</style>
