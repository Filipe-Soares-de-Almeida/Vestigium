<script lang="ts">
  import { app } from "../../lib/app.svelte";
  import { errorText } from "../../lib/errors";
  import { lang, langFor, t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import type { LanguagePref, Settings } from "../../lib/types";
  import Row from "./Row.svelte";
  import { saveSettings } from "./data";

  let { settings }: { settings: Settings } = $props();

  const LANGUAGES: LanguagePref[] = ["System", "PtBr", "En"];

  /** The window keeps its language until it opens again; the tray follows at once. */
  async function choose(language: LanguagePref): Promise<void> {
    try {
      await saveSettings({ language });
      if (langFor(language) !== lang()) toast.show(t("ui.settings.language.later"));
    } catch (error) {
      toast.show(errorText(error));
    }
  }
</script>

<Row id="language" title={t("ui.settings.language.section")} hint={t("ui.settings.language.app.hint")}>
  <select
    value={settings.language}
    disabled={!app.editable}
    aria-labelledby="language-name"
    aria-describedby="language-hint"
    data-focus="language"
    onchange={(event) => choose(event.currentTarget.value as LanguagePref)}
  >
    {#each LANGUAGES as language (language)}
      <option value={language}>{t(`ui.settings.language.${language}`)}</option>
    {/each}
  </select>
</Row>
