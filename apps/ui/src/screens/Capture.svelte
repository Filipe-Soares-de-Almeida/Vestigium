<script lang="ts">
  import { onMount } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import Steps from "../components/Steps.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { captures } from "../lib/capture.svelte";
  import { formatDuration } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { go, href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { CaptureDto, OccurrenceSummary } from "../lib/types";
  import { copyText } from "./occurrence/browser";
  import Alignment from "./capture/Alignment.svelte";
  import Apps from "./capture/Apps.svelte";
  import Coverage from "./capture/Coverage.svelte";
  import Drivers from "./capture/Drivers.svelte";
  import { buildResult, resultMarkdown } from "./capture/words";
  import OccurrenceRow from "./today/OccurrenceRow.svelte";

  let { id }: { id: number } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<CaptureDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);
  /** Reports from the minutes this capture ran. */
  let during = $state<OccurrenceSummary[]>([]);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const view = $derived(dto ? buildResult(dto, level) : null);
  const canStart = $derived(app.running && captures.run === null && !captures.asking);

  relatedLinks(() => [
    ...during.slice(0, 1).map((o) => ({ label: t("ui.related.back_report"), href: href.occurrence(o.id) })),
    { label: t("ui.related.new_capture"), href: href.captures() },
    { label: t("ui.related.events_today"), href: href.events() },
  ]);

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

<BackLink href={href.captures()} label={t("ui.captures.title")} />

{#if failure !== null}
  <div class="failure">
    <ErrorView error={failure} heading="h1" onretry={load} />
  </div>
{:else if dto === undefined}
  <h1 class="sr">{t("ui.captures.crumb")}</h1>
  <Loading />
{:else if !dto || !view}
  <PageHead title={t("ui.captures.missing.title")} sub={t("ui.captures.missing.body")} />
  <div class="acts"><a class="btn" href={href.captures()}>{t("ui.captures.missing.back")}</a></div>
{:else}
  {#snippet copyButton()}
    <button type="button" class="btn" onclick={copy}><Icon name="copy" />{t("ui.captures.copy")}</button>
  {/snippet}
  <PageHead title={view.title} sub={view.meta} actions={copyButton} />
  <div class="facts">
    <div class="fact">
      <span class="fact-l">{t("ui.occurrence.fact.duration")}</span>
      <span class="fact-v">
        {view.minutes < 60 ? t("ui.captures.dur.name", { minutes: view.minutes }) : formatDuration(view.minutes * 60_000)}
        {t("ui.captures.figure.caption")}
      </span>
    </div>
    <div class="fact">
      <span class="fact-l">{t("ui.captures.fact.result")}</span>
      <span class="fact-v">
        <StatusMark kind={view.none ? "ok" : "critical"} word={t(view.none ? "ui.captures.mark.quiet" : "ui.captures.mark.found")} />
      </span>
    </div>
    <div class="fact">
      <span class="fact-l">{t("ui.captures.fact.coverage")}</span>
      <span class="fact-v"><StatusMark kind={view.cover.mark.kind} word={view.cover.mark.word} /></span>
    </div>
  </div>

  {#if view.notes.length}
    <ul class="notes">
      {#each view.notes as note, i (i)}
        <li role="note">
          <Icon name={note.icon} />
          <span><b>{note.head}</b> {note.body}{#if note.reason}<code>{note.reason}</code>{/if}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <Block title={t("ui.captures.cover.title")}>
    <Coverage rows={view.cover.rows} />
  </Block>

  <div class="cols2">
    <Block title={t("ui.captures.conclusion.title")}>
      {#if view.main}
        <p class="cz-n">{view.main.name}</p>
        <p class="cz-m">{view.main.sub}</p>
        <p class="p">{view.main.summary}</p>
        <ul class="ev-l">
          {#each view.main.evidence as line, i (i)}
            <li>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</li>
          {/each}
        </ul>
      {:else}
        <p class="cz-n">{t("ui.captures.none.head")}</p>
        <p class="cz-m">{view.measured}</p>
        <p class="p">{t("ui.captures.none.body")}</p>
        <p class="note">{t("ui.captures.none.hint")}</p>
        <div class="acts">
          {#if canStart && view.minutes < 10}
            <button type="button" class="btn accent" onclick={again}>
              <Icon name="start" />{t("ui.captures.none.ten")}
            </button>
          {/if}
          <a class="btn" href={href.captures()}>{t("ui.captures.none.later")}</a>
        </div>
      {/if}
    </Block>

    <div>
      <Block title={t("ui.captures.todo.title")}>
        {#if view.steps.length}
          <Steps steps={view.steps} />
        {:else}
          <p class="empty">{t("ui.captures.todo.none")}</p>
        {/if}
      </Block>

      {#if during.length}
        <Block title={t("ui.captures.during.title")}>
          <RowList>
            {#each during as o (o.id)}
              <li><OccurrenceRow {o} {level} /></li>
            {/each}
          </RowList>
        </Block>
      {/if}
    </div>
  </div>

  {#if view.hasMarks}
    <Alignment {dto} {level} summary={view.alignSummary} title={t("ui.captures.align.title")} />
  {:else}
    <Block title={t("ui.captures.align.title")}>
      <p class="empty">{view.alignSummary}</p>
    </Block>
  {/if}

  {#if view.drivers && view.drivers.rows.length}
    <Block title={t("ui.captures.drivers.title")}>
      <Drivers table={view.drivers} {level} captureId={dto.id} />
    </Block>
  {/if}

  {#if view.apps}
    <Block title={t("ui.captures.apps.title")}>
      <Apps rows={view.apps.rows} quiet={view.apps.quiet} often={view.apps.often} all={dto.apps} {level} />
    </Block>
  {/if}
{/if}

<style>
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 52px;
    margin: 22px 0 0;
  }
  .fact {
    display: grid;
    gap: 3px;
  }
  .fact-l {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .fact-v {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 15px;
  }
  .fact-v :global(.status) {
    color: var(--ink);
    font-size: 15px;
  }
  .notes {
    display: grid;
    gap: 8px;
    max-width: 76ch;
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
  .notes b {
    color: var(--ink);
    font-weight: 600;
  }
  .notes code {
    display: block;
    margin-top: 4px;
    font: 12.5px var(--mono);
  }
  .cz-n {
    margin: 0;
    font: 600 16px/1.35 var(--f);
  }
  .cz-m {
    margin: 2px 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .ev-l small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .failure {
    margin-top: 14px;
  }
</style>
