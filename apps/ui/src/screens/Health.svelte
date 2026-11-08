<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import HistList from "../components/HistList.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark, { type StatusKind } from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { here, href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import { areaOf, AREAS, areaState, findArea, type Mark } from "./health/areas";
  import Finding from "./health/Finding.svelte";
  import Scan from "./health/Scan.svelte";
  import { formatDayShort } from "./occurrence/format";
  import { checkText, findingTitle, lede, when } from "./health/words";

  let { area: areaId }: { area: string | null } = $props();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const area = $derived(findArea(areaId));
  const states = $derived(health.dto ? AREAS.map((a) => areaState(a, health.dto!)) : []);
  const current = $derived(area && health.dto ? areaState(area, health.dto) : null);
  const checks = $derived(states.reduce((n, s) => n + s.checks.filter((c) => c.mark === "Ok").length, 0));
  const worst: StatusKind = $derived(health.critical ? "critical" : health.attention ? "attention" : "ok");

  const statusOf = (mark: Mark): StatusKind =>
    mark === "Critical" ? "critical" : mark === "Attention" ? "attention" : mark === "Unknown" ? "info" : "ok";
  const markWord = (mark: Mark): string => t(`ui.health.mark.${mark}`);

  /** The latest of what happened: alerts that opened, alerts that closed and the last full scan. */
  const history = $derived.by(() => {
    const dto = health.dto;
    if (!dto) return [];
    const entries: { at: number; text: string; href: string | null }[] = [
      ...dto.open.map((f) => ({
        at: f.opened_ms,
        text: t(f.concern === "Critical" ? "ui.health.history.new_critical" : "ui.health.history.new", {
          title: findingTitle(f, level),
        }),
        href: href.health(areaOf(f.kind).id),
      })),
      ...dto.closed.map((f) => ({
        at: f.closed_ms ?? f.opened_ms,
        text: t("ui.health.history.resolved", { title: findingTitle(f, level) }),
        href: href.detail("finding", f.id, { from: here(), label: t("ui.health.title") }),
      })),
      ...(dto.last_scan_ms ? [{ at: dto.last_scan_ms, text: t("ui.health.history.scan"), href: null }] : []),
    ];
    return entries.sort((a, b) => b.at - a.at).slice(0, 6);
  });

  relatedLinks(() =>
    area
      ? [
          ...(area.id === "disks" ? [{ label: t("ui.related.test_disks"), href: href.tests() }, { label: t("ui.group.disks"), href: href.disks() }] : []),
          ...(area.id === "memory" ? [{ label: t("ui.item.memory_tests"), href: href.tests("memory") }] : []),
          ...(area.id === "falls" ? [{ label: t("ui.related.dumps"), href: href.dumps() }] : []),
          { label: t("ui.related.health_overview"), href: href.health() },
        ]
      : [
          { label: t("ui.group.disks"), href: href.disks() },
          { label: t("ui.group.tests"), href: href.tests() },
          { label: t("ui.related.tweaks"), href: href.tweaks() },
        ],
  );

  // A disk point shows the last full read of that disk, kept by the Tests page.
  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });

  function retry(): void {
    health.error = null;
    void health.load();
  }
</script>

{#snippet areaMeta()}
  {#if current}
    {@const kind = statusOf(current.mark)}
    <StatusMark {kind} word={markWord(current.mark)} />
    {#if current.open.length}<span>{tn("ui.health.area.points", current.open.length)}</span>{/if}
  {/if}
{/snippet}

{#if area}
  <PageHead title={t(`ui.health.area.${area.id}`)} meta={current ? areaMeta : undefined} />
{:else}
  <PageHead
    title={t("ui.health.title")}
    sub={health.dto?.last_scan_ms ? t("ui.health.scan.last", { when: when(health.dto.last_scan_ms) }) : t("ui.health.scan.never")}
  />
{/if}

{#if health.dto === null && health.error !== null}
  <div class="gap"><ErrorView error={health.error} heading="h2" onretry={retry} /></div>
{:else if health.dto === null}
  <Loading />
{:else if current}
  {#each current.open as finding, i (finding.id)}
    <Finding {finding} {level} first={i === 0} />
  {/each}
  {#if current.open.length === 0}
    <p class="lede">{t("ui.health.area.none")}</p>
  {/if}
  {#if current.checks.some((c) => c.mark === "Unknown")}
    <Scan {level} />
  {/if}
  <Block title={t("ui.health.area.ok_title")}>
    {#if current.checks.length}
      <ul class="okl">
        {#each current.checks as check (check.kind)}
          <li><StatusMark kind={check.mark === "Ok" ? "ok" : "info"} size={20} reveal /><span>{checkText(check)}</span></li>
        {/each}
      </ul>
    {:else}
      <p class="empty">{t("ui.health.area.ok_none")}</p>
    {/if}
  </Block>
{:else}
  {#if app.history === "empty"}
    <p class="lede">{t("ui.health.lede.first")}</p>
  {:else}
    <p class="lede">
      <span class="lead"><StatusMark kind={worst} word={markWord(worst === "critical" ? "Critical" : worst === "attention" ? "Attention" : "Ok")} /></span>
      {#each lede(health.dto, level, checks) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
    </p>
  {/if}
  <Block title={t("ui.health.areas_label")}>
    <RowList>
      {#each states as s (s.area.id)}
        {@const kind = statusOf(s.mark)}
        <RowCard
          href={href.health(s.area.id)}
          icon={s.area.icon}
          title={t(`ui.health.area.${s.area.id}`)}
          sub={s.open.length > 1
            ? t("ui.health.area.more", { title: findingTitle(s.open[0], level), count: s.open.length - 1 })
            : s.open.length
              ? findingTitle(s.open[0], level)
              : s.mark === "Unknown"
                ? t("ui.health.area.unknown")
                : tn("ui.health.area.fine", s.checks.length)}
        >
          {#snippet right()}
            <StatusMark {kind} reveal word={markWord(s.mark)} />
            <span class="count">{s.open.length ? tn("ui.health.area.points", s.open.length) : ""}</span>
          {/snippet}
        </RowCard>
      {/each}
    </RowList>
  </Block>
  <div class="cols2">
    <Scan {level} />
    <Block title={t("ui.health.history.title")}>
      {#if history.length}
        <HistList entries={history.map((e, i) => ({ key: String(i), date: formatDayShort(e.at), title: e.text, href: e.href }))} />
      {:else}
        <p class="empty">{t("ui.health.history.none")}</p>
      {/if}
    </Block>
  </div>
{/if}

<style>
  .gap {
    margin-top: 14px;
  }
  .lead {
    display: inline-flex;
    margin-right: 4px;
    vertical-align: 1px;
  }
  .count {
    min-width: 64px;
    text-align: right;
  }
  .okl {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    font-size: 14px;
    list-style: none;
  }
  .okl li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    line-height: 20px;
  }
</style>
