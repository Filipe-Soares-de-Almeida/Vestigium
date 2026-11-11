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
  import { formatTime } from "../lib/format";
  import { health } from "../lib/health.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { passState } from "../lib/monitor";
  import { relatedLinks } from "../lib/related.svelte";
  import { here, href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import { areaOf, AREAS, areaState, findArea, statusKindOf } from "./health/areas";
  import Finding from "./health/Finding.svelte";
  import Scan from "./health/Scan.svelte";
  import { formatDayShort } from "./occurrence/format";
  import { areaUnknownText, checkText, findingTitle, lede, markWord, when } from "./health/words";

  let { area: areaId }: { area: string | null } = $props();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const area = $derived(findArea(areaId));
  /** How the hourly check stands, from the watcher's last answer; nothing when it says nothing. */
  const pass = $derived(passState(app.link, Date.now()));
  const states = $derived(health.dto ? AREAS.map((a) => areaState(a, health.dto!, pass)) : []);
  const current = $derived(area && health.dto ? areaState(area, health.dto, pass) : null);
  const checks = $derived(states.reduce((n, s) => n + s.checks.filter((c) => c.mark === "Ok").length, 0));
  /** Some area is unknown because a check was not read or is out of date, not because it needs a scan. */
  const unverified = $derived(states.some((s) => s.mark === "Unknown" && s.why !== null && s.why !== "scan"));
  const worst: StatusKind = $derived(
    health.critical ? "critical" : health.attention ? "attention" : pass?.silent ? "silent" : unverified ? "unverified" : "ok",
  );

  /** The line under the title: when the hourly check last finished, and a failed attempt after it. */
  const checkLine = $derived.by((): string | null => {
    const monitor = app.monitor;
    if (monitor === null || monitor.health_attempt_ms === null) return null;
    const { health_attempt_ms: attempt, health_ok_ms: ok } = monitor;
    if (ok !== null && ok >= attempt) return t("ui.health.scan.last_ok", { when: when(ok) });
    if (ok === null) return t("ui.health.scan.never_finished", { attempt: formatTime(attempt) });
    return t("ui.health.scan.failed", { when: when(ok), attempt: formatTime(attempt) });
  });

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
    {@const kind = statusKindOf(current.mark, current.why)}
    <StatusMark {kind} word={markWord(current.mark, current.why)} />
    {#if current.open.length}<span>{tn("ui.health.area.points", current.open.length)}</span>{/if}
  {/if}
{/snippet}

{#if area}
  <PageHead title={t(`ui.health.area.${area.id}`)} meta={current ? areaMeta : undefined} />
{:else}
  <PageHead
    title={t("ui.health.title")}
    sub={checkLine ?? (health.dto?.last_scan_ms ? t("ui.health.scan.last", { when: when(health.dto.last_scan_ms) }) : t("ui.health.scan.never"))}
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
  {#if current.checks.some((c) => c.why === "scan")}
    <Scan {level} />
  {/if}
  <Block title={t("ui.health.area.ok_title")}>
    {#if current.checks.length}
      <ul class="okl">
        {#each current.checks as check (check.kind)}
          <li><StatusMark kind={statusKindOf(check.mark, check.why)} size={20} reveal /><span>{checkText(check, pass)}</span></li>
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
      <span class="lead"><StatusMark kind={worst} word={worst === "silent" || worst === "unverified" ? undefined : markWord(worst === "critical" ? "Critical" : worst === "attention" ? "Attention" : "Ok")} /></span>
      {#each lede(health.dto, level, checks, pass) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
    </p>
  {/if}
  <Block title={t("ui.health.areas_label")}>
    <RowList>
      {#each states as s (s.area.id)}
        {@const kind = statusKindOf(s.mark, s.why)}
        <RowCard
          href={href.health(s.area.id)}
          icon={s.area.icon}
          title={t(`ui.health.area.${s.area.id}`)}
          sub={s.open.length > 1
            ? t("ui.health.area.more", { title: findingTitle(s.open[0], level), count: s.open.length - 1 })
            : s.open.length
              ? findingTitle(s.open[0], level)
              : s.mark === "Unknown"
                ? areaUnknownText(s.why, pass)
                : tn("ui.health.area.fine", s.checks.length)}
        >
          {#snippet right()}
            <StatusMark {kind} reveal word={markWord(s.mark, s.why)} />
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
