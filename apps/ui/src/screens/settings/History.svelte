<script lang="ts">
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

<Row
  id="retention"
  title={t("ui.settings.history.keep")}
  hint={t("ui.settings.history.keep.hint", { size: formatSize(total) })}
  wide
>
  <div class="fields">
    {#each FIELDS as { field, key } (field)}
      {@const wrong = rejected?.field === field ? rejected : null}
      <div class="field">
        <label for="retention-{key}">{t(`ui.settings.history.${key}`)}</label>
        <select
          id="retention-{key}"
          value={String(shown[field])}
          disabled={!app.editable}
          aria-invalid={wrong ? true : undefined}
          aria-describedby={wrong ? `retention-${key}-error` : undefined}
          data-focus="retention:{field}"
          onchange={(event) => change(field, Number(event.currentTarget.value))}
        >
          {#each choices(field) as value (value)}
            <option value={String(value)}>{label(field, value)}</option>
          {/each}
        </select>
        {#if wrong}
          <span class="invalid" id="retention-{key}-error" role="status">{reason(field, wrong.error)}</span>
        {/if}
      </div>
    {/each}
  </div>
  <div class="bar" aria-hidden="true">
    {#each FIELDS as { field, tone } (field)}
      <i style:flex-grow={sizes[field]} style:background="var(--{tone})"></i>
    {/each}
  </div>
  <ul class="key">
    {#each FIELDS as { field, key, tone } (field)}
      <li>
        <span class="state-dot" style:background="var(--{tone})"></span>
        {t(`ui.settings.history.size.${key}`, { size: formatSize(sizes[field]) })}
      </li>
    {/each}
    <li class="total">{t("ui.settings.history.size.total")} <b>{formatSize(total)}</b></li>
  </ul>
</Row>

<style>
  .fields {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
    align-items: start;
  }
  .field {
    display: grid;
    gap: 6px;
    color: var(--ink2);
    font: 400 13px/1.4 var(--f);
  }
  .invalid {
    color: var(--ink);
    font-weight: 600;
  }
  .bar {
    display: flex;
    gap: 2px;
    height: 10px;
    margin-top: 14px;
  }
  .bar i {
    min-width: 4px;
    border-radius: 999px;
  }
  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
  }
  .key .state-dot {
    margin-right: 6px;
  }
  .total {
    margin-left: auto;
  }
  .total b {
    color: var(--ink);
    font-weight: 600;
  }
  @container content (max-width: 720px) {
    .fields {
      grid-template-columns: minmax(0, 1fr);
    }
    .total {
      margin-left: 0;
    }
  }
  @media (forced-colors: active) {
    .bar i {
      forced-color-adjust: none;
      background: CanvasText !important;
    }
  }
</style>
