<script lang="ts">
  import { tick } from "svelte";

  import HelperLine from "../../components/HelperLine.svelte";
  import Icon from "../../components/Icon.svelte";
  import { app } from "../../lib/app.svelte";
  import { captures } from "../../lib/capture.svelte";
  import { focusHere } from "../../lib/focus";
  import { t } from "../../lib/i18n.svelte";

  let { context }: { context: string | null } = $props();

  const LENGTHS = [2, 5, 10] as const;
  const MIN_MINUTES = 1;
  /** A whole day, the longest capture the helper runs. */
  const MAX_MINUTES = 24 * 60;

  let choice = $state<(typeof LENGTHS)[number] | "custom">(5);
  /** What the two number fields hold; `null` while one is empty or not a number. */
  let hours = $state<number | null>(0);
  let mins = $state<number | null>(15);
  let field = $state<HTMLInputElement>();

  const custom = $derived(
    hours !== null && mins !== null && Number.isInteger(hours) && Number.isInteger(mins) && hours >= 0 && mins >= 0 && mins < 60
      ? hours * 60 + mins
      : null,
  );
  const customValid = $derived(custom !== null && custom >= MIN_MINUTES && custom <= MAX_MINUTES);
  const minutes = $derived(choice === "custom" ? custom : choice);
  const ready = $derived(choice !== "custom" || customValid);

  function pickCustom(): void {
    choice = "custom";
    void tick().then(() => field?.focus());
  }

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const measures = $derived(
    level === "Technical"
      ? "ui.captures.facts.measures.t"
      : level === "Detailed"
        ? "ui.captures.facts.measures.d"
        : "ui.captures.facts.measures",
  );
  const facts = $derived([
    { icon: "admin", text: t(level === "Technical" ? "ui.captures.facts.permission.t" : "ui.captures.facts.permission") },
    { icon: "play", text: t("ui.captures.facts.keep") },
    { icon: "chip", text: t(measures) },
    { icon: "shield", text: t("ui.captures.facts.local") },
  ] as const);
</script>

<section class="start" aria-label={t("ui.captures.start.title")}>
  <HelperLine job="capture" />
  {#if context}
    <p class="p context" tabindex="-1" use:focusHere>{context}</p>
  {/if}

  <div class="fld">
    <p class="fld-l" id="dur-label">{t("ui.captures.dur.label")}</p>
    <div class="rds v" role="radiogroup" aria-labelledby="dur-label">
      {#each LENGTHS as length (length)}
        <label class="rd">
          <input type="radio" name="cap-length" checked={choice === length} onchange={() => (choice = length)} />
          <span class="rd-c" aria-hidden="true"></span>
          <span class="rd-l">{t("ui.captures.dur.name", { minutes: length })}<small>{t(`ui.captures.dur.${length}.hint`)}</small></span>
        </label>
      {/each}
      <label class="rd">
        <input type="radio" name="cap-length" checked={choice === "custom"} onchange={pickCustom} />
        <span class="rd-c" aria-hidden="true"></span>
        <span class="rd-l">{t("ui.captures.dur.custom")}<small>{t("ui.captures.dur.custom.hint")}</small></span>
      </label>
    </div>
    {#if choice === "custom"}
      <div class="custom" role="group" aria-labelledby="dur-custom-label">
        <p class="fld-l" id="dur-custom-label">{t("ui.captures.dur.custom.label")}</p>
        <div class="entries">
          <label class="entry">
            <input
              id="dur-custom-hours"
              class="tf"
              type="number"
              inputmode="numeric"
              min="0"
              max="24"
              step="1"
              bind:value={hours}
              bind:this={field}
              aria-label={t("ui.captures.dur.custom.hours")}
              aria-invalid={!customValid}
              aria-describedby={customValid ? "dur-custom-help" : "dur-custom-help dur-custom-error"}
            />
            <span aria-hidden="true">{t("ui.captures.dur.custom.unit.h")}</span>
          </label>
          <label class="entry">
            <input
              id="dur-custom-minutes"
              class="tf"
              type="number"
              inputmode="numeric"
              min="0"
              max="59"
              step="1"
              bind:value={mins}
              aria-label={t("ui.captures.dur.custom.minutes")}
              aria-invalid={!customValid}
              aria-describedby={customValid ? "dur-custom-help" : "dur-custom-help dur-custom-error"}
            />
            <span aria-hidden="true">{t("ui.captures.dur.custom.unit")}</span>
          </label>
        </div>
        <p class="fld-n" id="dur-custom-help">{t("ui.captures.dur.custom.help")}</p>
        {#if !customValid}
          <p class="error" id="dur-custom-error" role="alert">{t("ui.captures.dur.custom.error")}</p>
        {/if}
      </div>
    {/if}
  </div>

  <ul class="notes">
    {#each facts as fact (fact.icon)}
      <li class:adm={fact.icon === "admin"}><Icon name={fact.icon} /><span>{fact.text}</span></li>
    {/each}
  </ul>

  <div class="acts">
    <button type="button" class="btn accent" disabled={!app.running || !ready}
      onclick={() => minutes !== null && captures.begin(minutes)}
    >
      <Icon name="start" />{t("ui.captures.start.button")}
    </button>
    {#if !app.running}
      <span class="muted">{t("ui.captures.start.needs_watcher")}</span>
    {/if}
  </div>
  <p class="note">{t("ui.captures.start.hint")}</p>
</section>

<style>
  .context {
    font-weight: 600;
    outline: none;
  }
  .custom {
    margin-top: 10px;
  }
  .entries {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--ink2);
    font-size: 13.5px;
  }
  .tf {
    width: 110px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--edge);
    border-radius: 6px;
    background: var(--field);
    box-shadow: inset 0 -1px 0 var(--edge);
    color: var(--ink);
    font: 400 14px/1.35 var(--f);
  }
  .tf:hover {
    border-color: var(--ink3);
  }
  .tf:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    outline: none;
  }
  .tf[aria-invalid="true"] {
    border-color: var(--accent);
  }
  .error {
    margin: 6px 0 0;
    color: var(--ink);
    font-size: 13px;
    font-weight: 600;
  }
  .notes {
    display: grid;
    gap: 8px;
    max-width: 64ch;
    margin: 18px 0 0;
    padding: 0;
    color: var(--ink2);
    font-size: 13.5px;
    list-style: none;
  }
  .notes li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .notes :global(svg) {
    flex: none;
    width: 16px;
    height: 16px;
    margin-top: 2px;
  }
  .notes .adm {
    align-items: center;
    padding: 8px 12px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 55%, transparent);
    color: var(--ink);
    font-weight: 600;
  }
  .notes .adm :global(svg) {
    margin-top: 0;
    color: var(--accent);
  }
  .muted {
    font-size: 13px;
  }
</style>
