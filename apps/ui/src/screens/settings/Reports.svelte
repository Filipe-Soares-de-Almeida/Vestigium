<script lang="ts">
  import Segmented from "../../components/Segmented.svelte";
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

<Row id="detail" title={t("ui.common.detail.label")} hint={t("ui.settings.reports.detail.hint")}>
  <Segmented
    label={t("ui.settings.reports.detail.label")}
    options={LEVELS.map((level) => ({
      value: level,
      label: t(`ui.common.detail.${level}`),
      hint: t(`ui.common.detail.${level}.hint`),
    }))}
    value={settings.detail_level}
    disabled={!app.editable}
    onchange={choose}
  />
</Row>
