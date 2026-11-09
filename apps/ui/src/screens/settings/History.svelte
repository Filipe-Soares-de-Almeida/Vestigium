<script lang="ts">
  import SelectField from "../../components/SelectField.svelte";
  import { app } from "../../lib/app.svelte";
  import { errorKind, errorText } from "../../lib/errors";
  import { t, tn } from "../../lib/i18n.svelte";
  import type { Retention, Settings } from "../../lib/types";
  import Row from "./Row.svelte";
  import {
    CHOICES,
    LIMITS,
    type RetentionField,
    formatSize,
    predictedBytes,
    saveSettings,
  } from "./data";

  let { settings }: { settings: Settings } = $props();

  const FIELDS: { field: RetentionField; key: string; tone: string }[] = [
    { field: "samples_hours", key: "samples", tone: "busy" },
    { field: "minutes_days", key: "minutes", tone: "calm" },
    { field: "records_days", key: "records", tone: "press" },
  ];

  /** A value the watcher turned down stays in its select, with the reason next to it. */
  let rejected = $state<{ field: RetentionField; value: number; error: unknown } | null>(null);

  const shown = $derived<Retention>(
    rejected ? { ...settings.retention, [rejected.field]: rejected.value } : settings.retention,
  );
  const sizes = $derived(predictedBytes(shown));
  const total = $derived(sizes.samples_hours + sizes.minutes_days + sizes.records_days);

  /** "24 hours", "2 days", "1 year": the largest whole unit. */
  function label(field: RetentionField, value: number): string {
    if (field === "samples_hours") {
      return value >= 48 && value % 24 === 0
        ? tn("ui.settings.history.days", value / 24)
        : tn("ui.settings.history.hours", value);
    }
    return value % 365 === 0
      ? tn("ui.settings.history.years", value / 365)
      : tn("ui.settings.history.days", value);
  }

  function choices(field: RetentionField): number[] {
    const values = new Set([...CHOICES[field], shown[field]]);
    return [...values].sort((a, b) => a - b);
  }

  function reason(field: RetentionField, error: unknown): string {
    if (errorKind(error) !== "refused") return errorText(error);
    const [min, max] = LIMITS[field];
    return t("ui.settings.history.invalid", { min: label(field, min), max: label(field, max) });
  }

  async function change(field: RetentionField, value: number): Promise<void> {
    rejected = null;
    try {
      await saveSettings({ retention: { ...settings.retention, [field]: value } });
    } catch (error) {
      rejected = { field, value, error };
    }
  }
</script>

<Row id="retention" title={t("ui.settings.history.by_kind")} wide>
  <div class="ret">
    {#each FIELDS as { field, key } (field)}
      {@const wrong = rejected?.field === field ? rejected : null}
      <div class="ret-r">
        <label class="ret-n" for="retention-{key}">{t(`ui.settings.history.${key}`)}</label>
        <SelectField
          full
          id="retention-{key}"
          value={String(shown[field])}
          options={choices(field).map((value) => ({ value: String(value), text: label(field, value) }))}
          disabled={!app.editable}
          aria-invalid={wrong ? true : undefined}
          aria-describedby={wrong ? `retention-${key}-error` : undefined}
          data-focus="retention:{field}"
          onchange={(value) => change(field, Number(value))}
        />
        <span class="ret-mb">{t("ui.settings.history.predicted", { size: formatSize(sizes[field]) })}</span>
        {#if wrong}
          <span class="invalid" id="retention-{key}-error" role="status">{reason(field, wrong.error)}</span>
        {/if}
      </div>
    {/each}
  </div>
</Row>
<Row id="retention-total" title={t("ui.settings.history.total_title")} wide>
  <p class="big">{formatSize(total)}</p>
  <p class="note">{t("ui.settings.history.total_note")}</p>
</Row>

<style>
  .ret {
    display: grid;
    gap: 14px;
    max-width: 760px;
  }
  .ret-r {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 220px 140px;
    gap: 8px 20px;
    align-items: center;
    font-size: 14px;
  }
  .ret-mb {
    color: var(--ink2);
    font-size: 13px;
    text-align: right;
  }
  .invalid {
    grid-column: 1 / -1;
    color: var(--ink);
    font-size: 13px;
    font-weight: 600;
  }
  .big {
    margin: 0;
    font: 350 34px/1.1 var(--fd);
  }
  @container content (max-width: 720px) {
    .ret-r {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .ret-mb {
      grid-column: 1 / -1;
      text-align: left;
    }
  }
</style>
