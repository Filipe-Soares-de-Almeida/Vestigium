<script lang="ts">
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { updates } from "../../lib/update.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";

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

</script>

<Row id="upd-auto" title={t("ui.settings.updates.section")} label={t("ui.settings.updates.auto")} hint={t("ui.settings.updates.auto_hint")}>
  <Toggle id="upd-auto" checked={updates.prefs?.check === true} onchange={(on) => updates.setCheck(on)} />
</Row>
<Row id="upd-version" title={t("ui.settings.updates.installed")} hint={t("ui.settings.updates.version_hint", { version: updates.version })} wide>
  <div class="acts">
    <button type="button" class="btn accent" disabled={updates.checking} onclick={checkNow}>
      {updates.result === "failed" ? t("ui.settings.updates.retry") : t("ui.settings.updates.check")}
    </button>
    {#if status}<span class="status" class:error={status.error} role="status">{status.text}</span>{/if}
  </div>
</Row>
<Row id="upd-news" title={t("ui.settings.updates.news")} wide>
  <RowList>
    <RowCard href={href.news()} icon="notes" title={t("ui.settings.updates.news_row")} />
  </RowList>
</Row>

<style>
  .acts {
    margin-top: 0;
  }
  .status {
    color: var(--ink2);
    font-size: 13px;
  }
  .status.error {
    color: var(--ink);
    font-weight: 600;
  }
</style>
