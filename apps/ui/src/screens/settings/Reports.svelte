<script lang="ts">
  import { app } from "../../lib/app.svelte";
  import { errorText } from "../../lib/errors";
  import { t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import type { DetailLevel, Settings } from "../../lib/types";
  import Row from "./Row.svelte";
  import { saveSettings } from "./data";

  let { settings }: { settings: Settings } = $props();

  const LEVELS: DetailLevel[] = ["Simple", "Detailed", "Technical"];

  async function choose(detail_level: DetailLevel): Promise<void> {
    try {
      await saveSettings({ detail_level });
    } catch (error) {
      toast.show(errorText(error));
    }
  }
</script>

<Row id="detail" title={t("ui.settings.reports.level")} hint={t("ui.settings.reports.detail.hint")} wide>
  <div class="rds v" role="radiogroup" aria-labelledby="detail-name">
    {#each LEVELS as level (level)}
      <label class="rd">
        <input
          type="radio"
          name="detail-level"
          checked={settings.detail_level === level}
          disabled={!app.editable}
          onchange={() => choose(level)}
        />
        <span class="rd-c" aria-hidden="true"></span>
        <span class="rd-l">{t(`ui.common.detail.${level}`)}<small>{t(`ui.common.detail.${level}.hint`)}</small></span>
      </label>
    {/each}
  </div>
</Row>
