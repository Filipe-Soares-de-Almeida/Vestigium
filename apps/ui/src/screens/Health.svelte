<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark, { type StatusKind } from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import { AREAS, areaState, findArea, type Mark } from "./health/areas";
  import Finding from "./health/Finding.svelte";
  import Scan from "./health/Scan.svelte";
  import { checkText, findingTitle, lede, when } from "./health/words";

  let { area: areaId }: { area: string | null } = $props();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const area = $derived(findArea(areaId));
  const states = $derived(health.dto ? AREAS.map((a) => areaState(a, health.dto!)) : []);
  const current = $derived(area && health.dto ? areaState(area, health.dto) : null);
  const checks = $derived(states.reduce((n, s) => n + s.checks.filter((c) => c.mark === "Ok").length, 0));
  const closed = $derived(health.dto?.closed ?? []);

  /** An area with no alert shows no status: its line already says the checks are fine. */
  const statusOf = (mark: Mark): StatusKind | null =>
    mark === "Critical" ? "critical" : mark === "Attention" ? "attention" : mark === "Unknown" ? "info" : null;

  // A disk point shows the last full read of that disk, kept by the Tests page.
  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });

  function retry(): void {
    health.error = null;
    void health.load();
  }
</script>

{#if area}
  <BackLink href={href.health()} label={t("ui.common.back_to", { place: t("ui.health.title") })} />
  <h1>{t(`ui.health.area.${area.id}`)}</h1>
{:else}
  <h1>{t("ui.health.title")}</h1>
{/if}

{#if health.dto === null && health.error !== null}
  <div class="gap"><ErrorView error={health.error} heading="h2" onretry={retry} /></div>
{:else if health.dto === null}
  <Loading />
{:else if current}
  {#each current.open as finding, i (finding.id)}
    <Finding {finding} {level} first={i === 0} />
  {/each}
  {#if current.checks.some((c) => c.mark === "Unknown")}
    <Scan {level} />
  {/if}
  <Block title={t("ui.health.area.ok_title")} first={current.open.length === 0}>
    {#each current.checks as check (check.kind)}
      <p class="check">
        <StatusMark kind={check.mark === "Ok" ? "ok" : "info"} size={20} reveal />
        <span>{checkText(check)}</span>
      </p>
    {:else}
      <p class="muted">{t("ui.health.area.ok_none")}</p>
    {/each}
  </Block>
  {#if current.closed.length}
    <Block title={t("ui.health.area.closed_title")}>
      {#each current.closed as finding (finding.id)}
        <p class="check">
          <StatusMark kind="ok" size={20} reveal />
          <span>
            {findingTitle(finding, level)}
            <small>
              {t("ui.health.closed_line", { opened: when(finding.opened_ms), closed: when(finding.closed_ms ?? 0) })}
            </small>
          </span>
        </p>
      {/each}
    </Block>
  {/if}
{:else}
  <p class="lede">
    {#if app.history === "empty"}
      {t("ui.health.lede.first")}
    {:else}
      {#each lede(health.dto, level, checks) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
    {/if}
  </p>
  <Block title={t("ui.health.areas_label")} first>
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
            {#if s.open.length}<span>{tn("ui.health.area.points", s.open.length)}</span>{/if}
            {#if kind}<StatusMark {kind} reveal word={s.mark === "Unknown" ? t("ui.health.mark.Unknown") : undefined} />{/if}
          {/snippet}
        </RowCard>
      {/each}
    </RowList>
  </Block>
  <Scan {level} />
  {#if closed.length}
    <Block title={t("ui.health.area.closed_title")}>
      {#each closed as finding (finding.id)}
        <p class="check">
          <StatusMark kind="ok" size={20} reveal />
          <span>
            {findingTitle(finding, level)}
            <small>
              {t("ui.health.closed_line", { opened: when(finding.opened_ms), closed: when(finding.closed_ms ?? 0) })}
            </small>
          </span>
        </p>
      {/each}
    </Block>
  {/if}
{/if}

<style>
  .gap {
    margin-top: 14px;
  }
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
  .lede b {
    color: var(--ink);
    font-weight: 600;
  }
  .check {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 12px;
    align-items: start;
    margin: 0;
    padding: 6px 0;
    font-size: 14px;
    line-height: 1.45;
  }
  .check small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .muted {
    margin: 0;
  }
</style>
