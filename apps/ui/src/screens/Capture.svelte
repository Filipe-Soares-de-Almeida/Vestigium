<script lang="ts">
  import { onMount } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import Steps from "../components/Steps.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { captures } from "../lib/capture.svelte";
  import { formatDuration, formatTime } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { go, href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { CaptureDto, OccurrenceSummary } from "../lib/types";
  import { copyText } from "./occurrence/browser";
  import Alignment from "./capture/Alignment.svelte";
  import Apps from "./capture/Apps.svelte";
  import Drivers from "./capture/Drivers.svelte";
  import { buildResult, resultMarkdown } from "./capture/words";
  import { toneOf } from "./today/day";
  import { causeIcon, occurrenceTitle } from "./today/text";

  let { id }: { id: number } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<CaptureDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);
  /** Reports from the minutes this capture ran. */
  let during = $state<OccurrenceSummary[]>([]);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const view = $derived(dto ? buildResult(dto, level) : null);
  const canStart = $derived(app.running && captures.run === null && !captures.asking);

  async function load(): Promise<void> {
    failure = null;
    dto = undefined;
    try {
      dto = await captures.get(id);
      if (dto) {
        captures.seen(id);
        const found = await marks(dto.start_ms, dto.end_ms).catch(() => null);
        during = found?.occurrences ?? [];
      }
    } catch (error) {
      failure = error;
    }
  }

  onMount(() => {
    void load();
  });

  async function copy(): Promise<void> {
    if (!dto) return;
    const copied = await copyText(resultMarkdown(dto));
    toast.show(t(copied ? "ui.captures.toast.copied" : "ui.captures.toast.copy_failed"));
  }

  /** Starts a ten-minute capture and shows the page where it runs. */
  function again(): void {
    void captures.begin(10);
    go(href.captures());
  }
</script>

<nav class="crumb" aria-label={t("ui.captures.path")}>
  <a href={href.captures()}>{t("ui.shell.nav.captures")}</a>
  <Icon name="chevron" />
  <span aria-current="page">{view?.crumb ?? t("ui.captures.crumb")}</span>
</nav>

{#if failure !== null}
  <div class="failure">
    <ErrorView error={failure} heading="h1" onretry={load} />
  </div>
{:else if dto === undefined}
  <h1 class="sr">{t("ui.captures.crumb")}</h1>
  <Loading />
{:else if !dto || !view}
  <section class="card missing">
    <h1>{t("ui.captures.missing.title")}</h1>
    <p>{t("ui.captures.missing.body")}</p>
    <a class="btn" href={href.captures()}>{t("ui.captures.missing.back")}</a>
  </section>
{:else}
  <div class="head">
    <div>
      <h1>{view.title}</h1>
      <p class="meta">{view.meta}</p>
      <div class="tools">
        <button type="button" class="btn" onclick={copy}><Icon name="copy" />{t("ui.captures.copy")}</button>
      </div>
    </div>
    <p class="figure">
      {view.minutes < 60 ? t("ui.captures.dur.name", { minutes: view.minutes }) : formatDuration(view.minutes * 60_000)}
      <small>{t("ui.captures.figure.caption")}</small>
    </p>
  </div>

  {#if view.notes.length}
    <div class="notes">
      {#each view.notes as note, i (i)}
        <div class="note" role="note">
          <Icon name={note.icon} />
          <div>
            <b>{note.head}</b>
            <span>{note.body}</span>
            {#if note.reason}<code>{note.reason}</code>{/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <div class="split">
    <section class="card" aria-labelledby="cap-cause">
      <h2 id="cap-cause" class="sub">{t("ui.captures.conclusion.title")}</h2>
      {#if view.main}
        <div class="cause">
          <span class="icochip"><Icon name={view.main.icon} /></span>
          <div>
            <b>{view.main.name}</b>
            <small>{view.main.sub}</small>
          </div>
        </div>
        <p class="summary">{view.main.summary}</p>
        <ul class="ev">
          {#each view.main.evidence as line, i (i)}
            <li>
              <span class="bullet" aria-hidden="true"></span>
              <span>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <div class="cause">
          <span class="icochip quiet"><Icon name="wave" /></span>
          <div>
            <b>{t("ui.captures.none.head")}</b>
            <small>{view.measured}</small>
          </div>
        </div>
        <p class="summary">{t("ui.captures.none.body")}</p>
        <p class="aside">{t("ui.captures.none.hint")}</p>
        <div class="actions">
          {#if canStart && view.minutes < 10}
            <button type="button" class="btn accent" onclick={again}>
              <Icon name="shield" />{t("ui.captures.none.ten")}
            </button>
          {/if}
          <a class="btn" href={href.captures()}>{t("ui.captures.none.later")}</a>
        </div>
      {/if}
    </section>

    <div class="stack">
      <section class="card" aria-labelledby="cap-todo">
        <h2 id="cap-todo" class="sub">{t("ui.captures.todo.title")}</h2>
        {#if view.steps.length}
          <Steps steps={view.steps} />
        {:else}
          <p class="muted">{t("ui.captures.todo.none")}</p>
        {/if}
      </section>

      {#if during.length}
        <section class="card list" aria-labelledby="cap-during">
          <h2 id="cap-during" class="sub">{t("ui.captures.during.title")}</h2>
          <ul>
            {#each during as o (o.id)}
              <li>
                <a class="row" href={href.occurrence(o.id)}>
                  <span class="icochip tone-{toneOf(o)}"><Icon name={causeIcon(o.cause)} /></span>
                  <span class="what"><b>{occurrenceTitle(o, level)}</b></span>
                  <span class="when">{formatTime(o.start_ms)}<br />{formatDuration(o.end_ms - o.start_ms)}</span>
                  <span class="chev"><Icon name="chevron" /></span>
                </a>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  </div>

  {#if view.drivers && view.drivers.rows.length}
    <section class="card wide" aria-labelledby="cap-drivers">
      <h2 id="cap-drivers" class="sub">{t("ui.captures.drivers.title")}</h2>
      <Drivers table={view.drivers} {level} />
    </section>
  {/if}

  <section class="card wide" aria-labelledby="cap-align">
    <h2 id="cap-align" class="sub">{t("ui.captures.align.title")}</h2>
    {#if view.hasMarks}
      <Alignment {dto} {level} summary={view.alignSummary} />
    {:else}
      <p class="muted flat">{view.alignSummary}</p>
    {/if}
  </section>

  {#if view.apps}
    <section class="card wide" aria-labelledby="cap-apps">
      <h2 id="cap-apps" class="sub">{t("ui.captures.apps.title")}</h2>
      <Apps rows={view.apps.rows} quiet={view.apps.quiet} all={dto.apps} {level} />
    </section>
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
  .figure small {
    display: block;
    margin-top: 4px;
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
    letter-spacing: 0;
  }
  .notes {
    display: grid;
    gap: 8px;
    margin-bottom: 14px;
  }
  .note {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    padding: 10px 14px;
    border: 1px solid var(--stroke2);
    border-radius: 12px;
    background: var(--card2);
    font-size: 13.5px;
    line-height: 1.45;
  }
  .note :global(svg) {
    margin-top: 1px;
    color: var(--ink2);
  }
  .note b {
    display: block;
    font-weight: 600;
  }
  .note span {
    color: var(--ink2);
  }
  .note code {
    display: block;
    margin-top: 4px;
    color: var(--ink2);
    font: 400 12.5px var(--f);
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
  .cause .icochip {
    width: 44px;
    height: 44px;
    background: var(--busy-t);
    color: var(--busy);
  }
  .cause .icochip.quiet {
    background: var(--card2);
    border: 1px solid var(--stroke2);
    color: var(--ink2);
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
  .aside {
    margin: 0 0 14px;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
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
    background: var(--accent);
  }
  .ev small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .list {
    padding: 10px;
  }
  .list .sub {
    padding: 6px 10px 2px;
  }
  .list ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto 14px;
    gap: 12px;
    align-items: center;
    padding: 9px 10px;
    border-radius: 8px;
    color: inherit;
    text-decoration: none;
  }
  .row:hover {
    background: var(--card2);
  }
  .what b {
    font-weight: 600;
    line-height: 1.3;
  }
  .when {
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.35;
    text-align: right;
    white-space: nowrap;
  }
  .chev {
    display: grid;
    color: var(--ink3);
  }
  .tone-busy {
    background: var(--busy-t);
    color: var(--busy);
  }
  .tone-press {
    background: var(--press-t);
    color: var(--press);
  }
  .tone-freeze {
    background: var(--freeze-t);
    color: var(--freeze);
  }
  .wide {
    margin-top: 14px;
  }
  .flat {
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
    .bullet {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
