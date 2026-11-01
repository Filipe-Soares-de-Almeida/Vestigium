<script lang="ts">
  import { onMount } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import Segmented from "../components/Segmented.svelte";
  import { occurrence } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { DetailLevel, OccurrenceDto } from "../lib/types";
  import { copyText, loadDone, saveDone } from "./occurrence/browser";
  import { chartView } from "./occurrence/chart";
  import { buildReport, type Line, reportMarkdown } from "./occurrence/report";
  import TechTables from "./occurrence/TechTables.svelte";
  import WindowChart from "./occurrence/WindowChart.svelte";

  let { id }: { id: number } = $props();

  const LEVELS: DetailLevel[] = ["Simple", "Detailed", "Technical"];

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<OccurrenceDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);
  /** The level picked here, until the saved setting catches up with it. */
  let picked = $state<DetailLevel | null>(null);
  let done = $state<string[]>([]);

  const level = $derived(picked ?? app.settings?.settings.detail_level ?? "Simple");
  const view = $derived(dto ? buildReport(dto, level) : null);
  const chart = $derived(dto && view ? chartView(dto, level, view.cause) : null);

  async function load(): Promise<void> {
    failure = null;
    dto = undefined;
    try {
      dto = await occurrence(id);
    } catch (error) {
      failure = error;
    }
  }

  onMount(() => {
    done = loadDone(id);
    void load();
  });

  /** Shows the level at once and saves it as the default while the watcher can write settings. */
  async function pickLevel(next: DetailLevel): Promise<void> {
    picked = next;
    const saved = app.settings?.settings;
    if (!app.editable || !saved || saved.detail_level === next) return;
    try {
      await app.saveSettings({ ...saved, detail_level: next });
    } catch (error) {
      toast.show(errorText(error));
    }
  }

  function tick(key: string, on: boolean): void {
    done = on ? [...done, key] : done.filter((k) => k !== key);
    saveDone(id, done);
  }

  async function copy(): Promise<void> {
    if (!dto) return;
    const copied = await copyText(reportMarkdown(dto));
    toast.show(t(copied ? "ui.occurrence.toast.copied" : "ui.occurrence.toast.copy_failed"));
  }
</script>

{#snippet evidence(lines: Line[], main: boolean)}
  <ul class="ev" class:main>
    {#each lines as line, i (i)}
      <li>
        <span class="bullet" aria-hidden="true"></span>
        <span>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</span>
      </li>
    {/each}
  </ul>
{/snippet}

<nav class="crumb" aria-label={t("ui.occurrence.path")}>
  <a href={href.today()}>{t("ui.shell.nav.today")}</a>
  <Icon name="chevron" />
  <span aria-current="page">{view?.crumb ?? t("ui.occurrence.crumb")}</span>
</nav>

{#if failure !== null}
  <div class="failure">
    <ErrorView error={failure} heading="h1" onretry={load} />
  </div>
{:else if dto === undefined}
  <h1 class="sr">{t("ui.occurrence.crumb")}</h1>
  <Loading />
{:else if !dto || !view || !chart}
  <section class="card missing">
    <h1>{t("ui.occurrence.missing.title")}</h1>
    <p>{t("ui.occurrence.missing.body")}</p>
    <a class="btn" href={href.today()}>{t("ui.occurrence.missing.back")}</a>
  </section>
{:else}
  <div class="head">
    <div>
      <h1>{view.title}</h1>
      <p class="meta">{view.meta}</p>
      <div class="tools">
        <Segmented
          label={t("ui.common.detail.label")}
          options={LEVELS.map((value) => ({
            value,
            label: t(`ui.common.detail.${value}`),
            hint: t(`ui.common.detail.${value}.hint`),
          }))}
          value={level}
          onchange={pickLevel}
        />
        <button type="button" class="btn" onclick={copy}><Icon name="copy" />{t("ui.occurrence.copy")}</button>
      </div>
    </div>
    <p class="figure">
      <span class="state-dot" style:background="var(--{view.tone})"></span>{view.figure}
      <small>{view.figureCaption}</small>
    </p>
  </div>

  <div class="split">
    <section class="card" aria-labelledby="occ-cause">
      <h2 id="occ-cause" class="sub">{t("ui.occurrence.cause.title")}</h2>
      <div class="cause">
        <span class="icochip" style:background="var(--{view.tone}-t)" style:color="var(--{view.tone})">
          <Icon name={view.main.icon} />
        </span>
        <div>
          <b>{view.main.name}</b>
          <small>{[view.main.sub, view.main.confidence].filter(Boolean).join(" · ")}</small>
        </div>
      </div>
      <p class="summary">{view.summary}</p>
      {@render evidence(view.main.evidence, true)}
      {#if view.note}
        <p class="note">{view.note}</p>
      {/if}
      {#each view.others as other, i (i)}
        <details class="also">
          <summary>{t("ui.occurrence.also", { cause: other.name, confidence: other.confidence })}</summary>
          {#if other.sub}<p class="also-sub">{other.sub}</p>{/if}
          {@render evidence(other.evidence, false)}
        </details>
      {/each}
    </section>

    <div class="stack">
      <section class="card" aria-labelledby="occ-todo">
        <h2 id="occ-todo" class="sub">{t("ui.occurrence.todo.title")}</h2>
        {#if view.steps.length}
          <div class="todo">
            {#each view.steps as step (step.key)}
              <label class="chk">
                <input
                  type="checkbox"
                  checked={done.includes(step.key)}
                  onchange={(event) => tick(step.key, event.currentTarget.checked)}
                />
                <span>{step.text}</span>
              </label>
            {/each}
          </div>
        {:else}
          <p class="muted none">{t("ui.occurrence.todo.none")}</p>
        {/if}
      </section>

      <section class="card" aria-labelledby="occ-seq">
        <h2 id="occ-seq" class="sub">{t("ui.occurrence.seq.title")}</h2>
        {#if view.facts.length}
          <ol class="seq">
            {#each view.facts as fact, i (i)}
              <li class:mark={fact.mark}>
                <time datetime={fact.datetime}>{fact.time}</time>
                <span>{fact.line.main}{#if fact.line.sub}<small>{fact.line.sub}</small>{/if}</span>
              </li>
            {/each}
          </ol>
        {:else}
          <p class="muted none">{t("ui.occurrence.seq.empty")}</p>
        {/if}
      </section>
    </div>
  </div>

  <section class="card chartcard" aria-labelledby="occ-chart">
    <h2 id="occ-chart" class="sub">{t("ui.occurrence.chart.title")}</h2>
    {#if dto.truncated}
      <p class="muted caption">{t("ui.occurrence.chart.truncated")}</p>
    {/if}
    {#if chart.lanes.length}
      <WindowChart view={chart} />
    {:else}
      <p class="muted none">{t("ui.occurrence.chart.empty")}</p>
    {/if}
  </section>

  {#if level === "Technical"}
    <TechTables {dto} />
  {/if}
{/if}

<style>
  .crumb {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font: 600 28px/1.25 var(--fd);
    letter-spacing: -0.005em;
  }
  .crumb a {
    color: var(--ink2);
    text-decoration: none;
  }
  .crumb a:hover {
    color: var(--ink);
  }
  .crumb :global(svg) {
    width: 16px;
    height: 16px;
    color: var(--ink3);
  }
  .head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    align-items: end;
    margin: 14px 0;
  }
  .head h1 {
    font: 600 22px/1.3 var(--fd);
    letter-spacing: 0;
  }
  .meta {
    margin: 4px 0 0;
    color: var(--ink2);
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-top: 12px;
  }
  .figure {
    margin: 0;
    font: 600 40px/1 var(--fd);
    letter-spacing: -0.02em;
    text-align: right;
    white-space: nowrap;
  }
  .figure .state-dot {
    width: 12px;
    height: 12px;
    margin-right: 10px;
    vertical-align: 6px;
  }
  .figure small {
    display: block;
    margin-top: 4px;
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
    letter-spacing: 0;
  }
  .sub {
    margin: 0 0 8px;
    font: 600 15px/1.3 var(--f);
  }
  .cause {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
    margin-bottom: 12px;
  }
  .cause b {
    display: block;
    font-size: 16.5px;
    font-weight: 600;
    line-height: 1.35;
  }
  .cause small {
    color: var(--ink2);
    font-size: 13px;
  }
  .summary {
    margin: 0 0 14px;
    font-size: 16.5px;
    line-height: 1.55;
  }
  .ev {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ev li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    line-height: 1.45;
  }
  .bullet {
    width: 6px;
    height: 6px;
    margin: 0.5em 0 0 6px;
    border-radius: 50%;
    background: var(--ink3);
  }
  .ev.main .bullet {
    background: var(--accent);
  }
  .ev small,
  .seq small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .note {
    margin: 12px 0 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .also {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--stroke2);
  }
  .also summary {
    border-radius: 6px;
    color: var(--ink2);
    font: 600 14px/1.45 var(--f);
    cursor: pointer;
  }
  .also summary:hover {
    color: var(--ink);
  }
  .also-sub {
    margin: 6px 0 8px;
    color: var(--ink2);
    font-size: 13px;
  }
  .also[open] summary {
    margin-bottom: 4px;
  }
  .todo {
    display: grid;
    gap: 10px;
  }
  .chk {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    line-height: 1.45;
    cursor: pointer;
  }
  .chk input {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: 1px 0 0;
    border: 1.5px solid var(--ink3);
    border-radius: 6px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .chk input:checked {
    border-color: var(--accent);
    background: var(--accent);
  }
  .chk input:checked::after {
    content: "";
    width: 9px;
    height: 5px;
    border-bottom: 2px solid var(--on-accent);
    border-left: 2px solid var(--on-accent);
    transform: rotate(-45deg) translate(1px, -1px);
  }
  .chk input:checked + span {
    color: var(--ink3);
    text-decoration: line-through;
  }
  .seq {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .seq li {
    display: grid;
    grid-template-columns: minmax(72px, auto) minmax(0, 1fr);
    gap: 12px;
    padding: 8px 0;
    border-top: 1px solid var(--stroke2);
  }
  .seq li:first-child {
    border-top: 0;
  }
  .seq time {
    padding-top: 1px;
    color: var(--ink2);
    font-size: 13px;
    font-weight: 600;
  }
  .seq li.mark > span {
    font-weight: 600;
  }
  .seq li.mark > span::before {
    content: "";
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 8px;
    border-radius: 50%;
    background: var(--freeze);
    vertical-align: 1px;
  }
  .seq li.mark small {
    font-weight: 400;
  }
  .chartcard {
    margin-top: 14px;
  }
  .caption {
    margin: -2px 0 10px;
    font-size: 13px;
  }
  .none {
    margin: 0;
  }
  .failure {
    margin-top: 14px;
  }
  .missing {
    max-width: 62ch;
    margin-top: 14px;
  }
  .missing h1 {
    font: 600 18px/1.3 var(--fd);
    margin: 0 0 6px;
  }
  .missing p {
    margin: 0 0 14px;
    color: var(--ink2);
  }
  @container content (max-width: 720px) {
    .head {
      grid-template-columns: minmax(0, 1fr);
      align-items: start;
    }
    .figure {
      text-align: left;
    }
  }
  @media (forced-colors: active) {
    .bullet,
    .seq li.mark > span::before {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .chk input:checked {
      forced-color-adjust: none;
      background: Highlight;
      border-color: Highlight;
    }
  }
</style>
