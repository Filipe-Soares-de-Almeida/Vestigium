<script lang="ts">
  import { tick } from "svelte";

  import Icon from "../../components/Icon.svelte";
  import { app } from "../../lib/app.svelte";
  import { captures } from "../../lib/capture.svelte";
  import { focusHere, radioGroup } from "../../lib/focus";
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

  /** Picks the custom tile; a click moves on to the field, the arrow keys stay on the tiles. */
  function pickCustom(event: MouseEvent): void {
    choice = "custom";
    if (event.detail > 0) void tick().then(() => field?.focus());
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
    { icon: "shield", text: t(level === "Technical" ? "ui.captures.facts.permission.t" : "ui.captures.facts.permission") },
    { icon: "play", text: t("ui.captures.facts.keep") },
    { icon: "chip", text: t(measures) },
    { icon: "pc", text: t("ui.captures.facts.local") },
  ] as const);
</script>

<section class="start" aria-labelledby="start-title">
  <h2 id="start-title" tabindex="-1" use:focusHere>{t("ui.captures.start.title")}</h2>
  {#if context}
    <p class="context">{context}</p>
  {/if}

  <div class="field">
    <p class="label" id="dur-label">{t("ui.captures.dur.label")}</p>
    <div class="tiles" role="radiogroup" aria-labelledby="dur-label" use:radioGroup>
      {#each LENGTHS as length (length)}
        <button
          type="button"
          role="radio"
          class="tile"
          aria-checked={choice === length}
          tabindex={choice === length ? 0 : -1}
          onclick={() => (choice = length)}
        >
          <b>{t("ui.captures.dur.name", { minutes: length })}</b>
          <span>{t(`ui.captures.dur.${length}.hint`)}</span>
        </button>
      {/each}
      <button
        type="button"
        role="radio"
        class="tile"
        aria-checked={choice === "custom"}
        tabindex={choice === "custom" ? 0 : -1}
        onclick={pickCustom}
      >
        <b>{t("ui.captures.dur.custom")}</b>
        <span>{t("ui.captures.dur.custom.hint")}</span>
      </button>
    </div>
    {#if choice === "custom"}
      <div class="custom" role="group" aria-labelledby="dur-custom-label">
        <p class="label" id="dur-custom-label">{t("ui.captures.dur.custom.label")}</p>
        <div class="entries">
          <div class="entry">
            <input
              id="dur-custom-hours"
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
          </div>
          <div class="entry">
            <input
              id="dur-custom-minutes"
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
          </div>
        </div>
        <p class="hint" id="dur-custom-help">{t("ui.captures.dur.custom.help")}</p>
        {#if !customValid}
          <p class="error" id="dur-custom-error" role="alert">{t("ui.captures.dur.custom.error")}</p>
        {/if}
      </div>
    {/if}
  </div>

  <ul class="facts">
    {#each facts as fact (fact.icon)}
      <li class:admin={fact.icon === "shield"}><Icon name={fact.icon} /><span>{fact.text}</span></li>
    {/each}
  </ul>

  <p class="hint">{t("ui.captures.start.hint")}</p>
  <div class="actions">
    <button type="button" class="btn accent" disabled={!app.running || !ready}
      onclick={() => minutes !== null && captures.begin(minutes)}
    >
      <Icon name="shield" />{t("ui.captures.start.button")}
    </button>
    {#if !app.running}
      <span class="hint">{t("ui.captures.start.needs_watcher")}</span>
    {/if}
  </div>
</section>

<style>
  .start {
    display: grid;
    gap: 14px;
  }
  .start {
    margin-top: 24px;
  }
  .start h2 {
    margin: 0;
    font: 400 20px/1.3 var(--fd);
    outline: none;
  }
  .facts li.admin {
    padding: 8px 12px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 55%, transparent);
    color: var(--ink);
    font-weight: 600;
  }
  .facts li.admin :global(svg) {
    color: var(--accent);
  }
  .context {
    margin: -8px 0 0;
    font-weight: 600;
  }
  .label {
    margin: 0 0 8px;
    color: var(--ink2);
    font: 600 13px/1.5 var(--f);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }
  .tile {
    display: grid;
    gap: 2px;
    padding: 12px 14px;
    border: 1px solid var(--stroke);
    border-radius: 10px;
    background: var(--card);
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }
  .tile:hover {
    background: var(--card2);
  }
  .tile b {
    font: 600 18px/1.3 var(--fd);
  }
  .tile span {
    color: var(--ink2);
    font-size: 13px;
  }
  .tile[aria-checked="true"] {
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--accent);
  }
  .custom {
    display: grid;
    justify-items: start;
    gap: 6px;
    margin-top: 12px;
  }
  .custom .label {
    margin: 0;
    color: var(--ink2);
    font: 600 13px/1.5 var(--f);
  }
  .entries {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink2);
  }
  .entry input {
    width: 80px;
    padding: 7px 10px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--card);
    color: var(--ink);
    font: 400 15px/1.4 var(--f);
  }
  .entry input[aria-invalid="true"] {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .error {
    margin: 0;
    color: var(--ink);
    font-size: 13px;
    font-weight: 600;
    line-height: 1.45;
  }
  .facts {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .facts li {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    line-height: 1.45;
  }
  .facts :global(svg) {
    margin-top: 1px;
    color: var(--ink2);
  }
  .hint {
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
  }
  @container content (max-width: 720px) {
    .tiles {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (forced-colors: active) {
    .tile[aria-checked="true"] {
      outline: 2px solid Highlight;
    }
  }
</style>
