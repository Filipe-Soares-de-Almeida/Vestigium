<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import StatusMark, { type StatusKind } from "../components/StatusMark.svelte";
  import Steps from "../components/Steps.svelte";
  import Toggle from "./settings/Toggle.svelte";
  import { occurrence } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { dumps } from "../lib/dumps.svelte";
  import { t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { OccurrenceDto } from "../lib/types";
  import { copyText } from "./occurrence/browser";
  import { chartView } from "./occurrence/chart";
  import { coveredLine } from "./capture/words";
  import { culpritView, dumpOfFall, fileName, hex } from "./dumps/words";
  import { formatBytes } from "./occurrence/format";
  import { buildReport, type Line, reportMarkdown } from "./occurrence/report";
  import TechTables from "./occurrence/TechTables.svelte";
  import WindowChart from "./occurrence/WindowChart.svelte";

  let { id, fromDay = null }: { id: number; fromDay?: string | null } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<OccurrenceDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const view = $derived(dto ? buildReport(dto, level) : null);
  const chart = $derived(dto && view ? chartView(dto, level, view.cause) : null);
  /** A fall keeps only what came before it, and its chart says so. */
  const fall = $derived(dto?.kind === "PostMortem" ? ".PostMortem" : "");
  /** The dump Windows wrote for a blue screen, once the dumps are read. */
  const fallDump = $derived.by(() => {
    const crash = dto?.triggers.flatMap((tr) => ("Crash" in tr ? [tr.Crash] : []))[0];
    if (!crash || typeof crash.kind !== "object" || !dumps.dto) return null;
    return dumpOfFall(dumps.dto, crash.down_ms, crash.kind.BlueScreen.code);
  });

  const MARK: Record<string, StatusKind> = { freeze: "critical", press: "overload", busy: "attention" };
  /** The tables of processes and Windows events; open from the start at the technical level. */
  let tech = $state(false);
  $effect(() => {
    tech = level === "Technical";
  });

  /** Where the cause leads: the disks for a disk, a capture for a driver, the live view for a program. */
  const topic = $derived.by(() => {
    const cause = view?.cause;
    if (cause === "Storage" || cause === "DiskIo") {
      return [
        { label: t("ui.related.disk_health"), href: href.health("disks") },
        { label: t("ui.related.test_disks"), href: href.tests() },
      ];
    }
    if (cause === "GpuDriver") {
      return [
        { label: t("ui.related.dumps"), href: href.dumps() },
        { label: t("ui.related.capture"), href: href.captures(dto?.id) },
      ];
    }
    if (cause === "Driver" || cause === "DriverModule" || cause === "Frames") {
      return [{ label: t("ui.related.capture"), href: href.captures(dto?.id) }];
    }
    return [{ label: t("ui.related.live"), href: href.activity("now") }];
  });

  relatedLinks(() => [
    ...topic,
    {
      label: t("ui.related.see_day"),
      href: fromDay ? href.history({ day: fromDay, month: fromDay.slice(0, 7) }) : href.activity(),
    },
  ]);

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
    void load();
  });

  async function copy(): Promise<void> {
    if (!dto) return;
    const copied = await copyText(reportMarkdown(dto));
    toast.show(t(copied ? "ui.occurrence.toast.copied" : "ui.occurrence.toast.copy_failed"));
  }
</script>

{#snippet evidence(lines: Line[])}
  <ul class="ev-l">
    {#each lines as line, i (i)}
      <li>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</li>
    {/each}
  </ul>
{/snippet}

<BackLink
  href={fromDay ? href.history({ day: fromDay, month: fromDay.slice(0, 7) }) : href.events()}
  label={fromDay ? t("ui.item.history") : t("ui.item.events")}
/>

{#if failure !== null}
  <div class="failure">
    <ErrorView error={failure} heading="h1" onretry={load} />
  </div>
{:else if dto === undefined}
  <h1 class="sr">{t("ui.occurrence.crumb")}</h1>
  <Loading />
{:else if !dto || !view || !chart}
  <PageHead title={t("ui.occurrence.missing.title")} sub={t("ui.occurrence.missing.body")} />
  <div class="acts"><a class="btn" href={href.activity()}>{t("ui.occurrence.missing.back")}</a></div>
{:else}
  {#snippet copyButton()}
    <button type="button" class="btn" onclick={copy}><Icon name="copy" />{t("ui.occurrence.copy")}</button>
  {/snippet}
  <PageHead title={view.title} sub={view.when} actions={copyButton} />
  <div class="facts">
    <div class="fact">
      <span class="fact-l">{t("ui.occurrence.fact.severity")}</span>
      <span class="fact-v"><StatusMark kind={MARK[view.tone]} word={view.severity} /></span>
    </div>
    <div class="fact">
      <span class="fact-l">{t("ui.occurrence.fact.duration")}</span>
      <span class="fact-v">{view.figure} {view.figureCaption}</span>
    </div>
    <div class="fact">
      <span class="fact-l">{t("ui.occurrence.fact.where")}</span>
      <span class="fact-v">{view.where}</span>
    </div>
  </div>

  <div class="cols2">
    <Block title={t("ui.occurrence.cause.title")}>
      <p class="cz-n">{view.main.name}</p>
      {#if view.main.sub || view.main.confidence}
        <p class="cz-m">{[view.main.sub, view.main.confidence].filter(Boolean).join(" · ")}</p>
      {/if}
      <p class="p">{view.summary}</p>
      {@render evidence(view.main.evidence)}
      {#if dto.tests.length}
        <p class="note">{t("ui.occurrence.during_test", { test: t(`ui.occurrence.test.${dto.tests[0].kind}`) })}</p>
      {/if}
      {#if view.others.length}
        <h3 class="h3">{t("ui.occurrence.also.title")}</h3>
        {#each view.others as other, i (i)}
          <p class="cz-n2">{other.name} <span class="cz-m">· {other.confidence}</span></p>
          {@render evidence(other.evidence)}
        {/each}
      {/if}
      {#if dto.capture}
        <p class="note">{coveredLine(dto.capture)}</p>
        <p class="more"><a href={href.capture(dto.capture.id)}>{t("ui.captures.covered.open")}</a></p>
      {:else if view.note}
        <p class="note">{view.note}</p>
        <p class="more"><a href={href.captures(dto.id)}>{t("ui.captures.entry")}</a></p>
      {/if}
    </Block>

    <div>
      <Block title={t("ui.occurrence.todo.title")}>
        {#if view.steps.length}
          <Steps steps={view.steps} />
        {:else}
          <p class="empty">{t("ui.occurrence.todo.none")}</p>
        {/if}
      </Block>

      {#if fallDump}
        {@const fault = fallDump.fault && "BugCheck" in fallDump.fault ? fallDump.fault.BugCheck : null}
        {@const culprit = fallDump.culprit ? culpritView(fallDump, fallDump.culprit, level) : null}
        <Block title={t("ui.occurrence.dump.title")}>
          {#if fallDump.analyzed_ms === null}
            <p class="cz-n">{t("ui.occurrence.dump.unread")}</p>
            <p class="cz-m">{fileName(fallDump.path)} · {formatBytes(fallDump.size_bytes)}</p>
          {:else}
            <p class="cz-n">{culprit?.name ?? t("ui.dumps.culprit.none")}</p>
            <p class="cz-m">{[fault ? hex(fault.code) : "", culprit?.confidence ?? ""].filter(Boolean).join(" · ")}</p>
          {/if}
          <p class="more">
            <a href={href.dumps(fallDump.id)}>
              {t(fallDump.analyzed_ms === null ? "ui.occurrence.dump.read" : "ui.occurrence.dump.open")}
            </a>
          </p>
        </Block>
      {/if}
    </div>
  </div>

  {#if chart.lanes.length}
    <WindowChart view={chart} title={t(`ui.occurrence.chart.title${fall}`)}>
      {#if dto.truncated}<p class="note">{t(`ui.occurrence.chart.truncated${fall}`)}</p>{/if}
    </WindowChart>
  {:else}
    <Block title={t(`ui.occurrence.chart.title${fall}`)}>
      <p class="empty">{t("ui.occurrence.chart.empty")}</p>
    </Block>
  {/if}

  <div class="cols2">
    <Block title={t("ui.occurrence.seq.title")}>
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
        <p class="empty">{t("ui.occurrence.seq.empty")}</p>
      {/if}
    </Block>
    <Block title={t("ui.occurrence.tech.block")}>
      <div class="fld">
        <p class="fld-l">{t("ui.occurrence.tech.show")}</p>
        <Toggle id="occ-tech" checked={tech} label={t("ui.occurrence.tech.show")} onchange={(on) => (tech = on)} />
      </div>
      {#if tech}<TechTables {dto} />{/if}
    </Block>
  </div>
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
    font-size: 15px;
  }
  .fact-v :global(.status) {
    color: var(--ink);
    font-size: 15px;
  }
  .cz-n {
    margin: 0;
    font: 600 16px/1.35 var(--f);
  }
  .cz-n2 {
    margin: 0 0 6px;
    font-size: 14px;
    font-weight: 600;
  }
  .cz-m {
    margin: 2px 0 10px;
    color: var(--ink2);
    font-size: 13px;
    font-weight: 400;
  }
  .ev-l small,
  .seq small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .h3 {
    margin: 20px 0 8px;
  }
  .seq {
    display: grid;
    margin: 0;
    padding: 0;
    font-size: 14px;
    list-style: none;
  }
  .seq li {
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr);
    gap: 12px;
    padding: 5px 0;
  }
  .seq time {
    color: var(--ink2);
    font-size: 13px;
  }
  .seq li.mark > span {
    font-weight: 600;
  }
  .seq li.mark > span::before {
    content: "";
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 9px;
    border-radius: 50%;
    background: var(--freeze);
    vertical-align: 1px;
  }
  .seq li.mark small {
    font-weight: 400;
  }
  .fld {
    margin: 0 0 18px;
  }
  .fld-l {
    margin: 0 0 8px;
    font-size: 14px;
  }
  .failure {
    margin-top: 14px;
  }
  @media (forced-colors: active) {
    .seq li.mark > span::before {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
