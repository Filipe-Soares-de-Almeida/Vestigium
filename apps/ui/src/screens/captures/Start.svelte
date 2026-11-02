<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { app } from "../../lib/app.svelte";
  import { captures } from "../../lib/capture.svelte";
  import { focusHere, radioGroup } from "../../lib/focus";
  import { t } from "../../lib/i18n.svelte";

  let { context }: { context: string | null } = $props();

  const LENGTHS = [2, 5, 10] as const;

  let minutes = $state<(typeof LENGTHS)[number]>(5);

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

<section class="card start" aria-labelledby="start-title">
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
          aria-checked={minutes === length}
          tabindex={minutes === length ? 0 : -1}
          onclick={() => (minutes = length)}
        >
          <b>{t("ui.captures.dur.name", { minutes: length })}</b>
          <span>{t(`ui.captures.dur.${length}.hint`)}</span>
        </button>
      {/each}
    </div>
  </div>

  <ul class="facts">
    {#each facts as fact (fact.icon)}
      <li><Icon name={fact.icon} /><span>{fact.text}</span></li>
    {/each}
  </ul>

  <p class="hint">{t("ui.captures.start.hint")}</p>
  <div class="actions">
    <button type="button" class="btn accent" disabled={!app.running} onclick={() => captures.begin(minutes)}>
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
  .start h2 {
    margin: 0;
    outline: none;
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
    grid-template-columns: repeat(3, minmax(0, 1fr));
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
