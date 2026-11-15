<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../../components/BackLink.svelte";
  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import PageHead from "../../components/PageHead.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import ZoomBar from "../../components/ZoomBar.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { startup } from "../../lib/startup.svelte";
  import type { DetailLevel } from "../../lib/types";
  import type { Zoom } from "../../lib/zoom.svelte";
  import BootPhasesChart from "./BootPhasesChart.svelte";
  import {
    bootKind,
    formatSeconds,
    judge,
    neighbours,
    type Phase,
    PHASES,
    phaseMs,
    profileAfterTpmMs,
    rangeText,
    runText,
    startupWhen,
    tempIsLarge,
    tempSub,
    totalText,
    tpmText,
    valueText,
  } from "./startup";

  /** The page of one startup of the Startup area: its phases on a time axis, what the profile waited for, its apps and its failures. */
  let { boot: start, level }: { boot: number; level: DetailLevel } = $props();

  const dto = $derived(startup.dto);
  const error = $derived(startup.error);
  const boots = $derived(dto?.boots ?? []);
  const found = $derived(neighbours(boots, start));
  const boot = $derived(found.boot);
  /** Only the latest startup has the Temp count that belongs to it. */
  const latest = $derived(boots.length > 0 && boots[boots.length - 1].kernel_start_ms === start);
  const kind = $derived(boot === null ? null : bootKind(boot));

  let zoom = $state<Zoom>();
  let plot = $state<HTMLElement>();
  const range = $derived(
    zoom ? t("ui.zoom.range", { from: formatSeconds(zoom.from * 1000), to: formatSeconds((zoom.from + zoom.span) * 1000) }) : "",
  );

  onMount(() => {
    void startup.load();
  });

  const k = (key: string, args?: Record<string, string | number>) => t(`ui.health.startup.${key}`, args);
  const name = (p: Phase) => k(`phase.${p.id}`);
</script>

<BackLink href={href.health("startup")} label={t("ui.health.area.startup")} />

{#if dto === null && error !== null}
  <ErrorView {error} heading="h1" onretry={() => startup.load()} />
{:else if dto === null}
  <h1 class="sr">{t("ui.health.area.startup")}</h1>
  <Loading />
{:else if boot === null}
  <PageHead title={k("boot.missing.title")} sub={k("boot.missing.body")} />
{:else}
  <PageHead title={k("boot.title", { when: startupWhen(start) })}>
    {#snippet meta()}
      {@const verdict = judge(boot, "profile")}
      <StatusMark kind={verdict.kind} word={verdict.word} />
      <span>{totalText(boot)}</span>
      {#if kind}<span>{k(`boot.type.${kind}`)}</span>{/if}
    {/snippet}
  </PageHead>

  <Block title={k("boot.phases")} first>
    {#snippet actions()}
      {#if zoom}<ZoomBar {zoom} {plot} {range} />{/if}
    {/snippet}
    <BootPhasesChart {boot} bind:zoom bind:plot />
    <div class="rows">
      <RowList>
        {#each PHASES as p (p.id)}
          {@const ms = phaseMs(boot, p.id)}
          {@const verdict = judge(boot, p.id)}
          <RowCard static chevron={false} icon={p.icon} title={name(p)} sub={k(`phase.${p.id}.sub`)}>
            {#snippet right()}
              <span class="val">
                <b class:none={ms === null}>{valueText(ms)}</b>
                <small>{rangeText(p.id)}</small>
              </span>
              <StatusMark kind={verdict.kind} word={verdict.word} reveal />
            {/snippet}
          </RowCard>
        {/each}
      </RowList>
    </div>
    {#if level === "Technical"}
      <span class="src">{k("boot.sources", { names: PHASES.map((p) => p.source).join(", ") })}</span>
      {#if boot.logger_full}<p class="note">{k("boot.logger_full")}</p>{/if}
    {/if}
  </Block>

  <Block title={k("boot.profile")}>
    <p class="p">{tpmText(boot) ?? k("boot.tpm.none")}</p>
    {#if profileAfterTpmMs(boot) !== null}
      <p class="p">{k("boot.tpm.after", { time: formatSeconds(profileAfterTpmMs(boot)!) })}</p>
    {/if}
    {#if latest}
      <RowList>
        <RowCard static chevron={false} icon="trash" title={k("temp.title")} sub={tempSub(dto.temp)}>
          {#snippet right()}
            {#if tempIsLarge(dto.temp)}<StatusMark kind="info" word={k("temp.large")} reveal />{/if}
          {/snippet}
        </RowCard>
      </RowList>
    {/if}
  </Block>

  <Block title={k("apps")}>
    {#if boot.apps.length}
      <RowList>
        {#each boot.apps as run (run.start_ms + run.command)}
          <RowCard static chevron={false} icon="app" title={run.command} sub={runText(run, boot)} />
        {/each}
      </RowList>
    {:else}
      <p class="empty">{k("body.unmeasured")}</p>
    {/if}
  </Block>

  <Block title={k("fail.title")}>
    {#if boot.failed_drivers.length}
      <RowList>
        {#each boot.failed_drivers as driver (driver)}
          {#if driver.toLowerCase() === "dam"}
            <RowCard title={k("fail.row", { name: driver })} sub={k("boot.fail.sub")}>
              {#snippet lead()}<StatusMark kind="info" size={20} bare />{/snippet}
              {#snippet expand()}
                <p class="p">{k("fail.dam")}</p>
                {#if level === "Technical"}<span class="src">{k("source", { name: "Service Control Manager 7026" })}</span>{/if}
              {/snippet}
            </RowCard>
          {:else}
            <RowCard static chevron={false} title={k("fail.row", { name: driver })} sub={k("boot.fail.sub")}>
              {#snippet lead()}<StatusMark kind="info" size={20} bare />{/snippet}
            </RowCard>
          {/if}
        {/each}
      </RowList>
    {:else}
      <p class="empty">{k("boot.fail.none")}</p>
    {/if}
  </Block>

  <nav class="pn" aria-label={k("boot.nav")}>
    {#if found.prev}
      <a href={href.startupBoot(found.prev.kernel_start_ms)}>{k("boot.prev", { when: startupWhen(found.prev.kernel_start_ms) })}</a>
    {:else}
      <span></span>
    {/if}
    {#if found.next}
      <a href={href.startupBoot(found.next.kernel_start_ms)}>{k("boot.next", { when: startupWhen(found.next.kernel_start_ms) })}</a>
    {/if}
  </nav>
{/if}

<style>
  .rows {
    margin-top: 18px;
  }
  .val {
    display: grid;
    justify-items: end;
    min-width: 108px;
  }
  .val b {
    font: 600 15px/1.3 var(--f);
  }
  .val b.none {
    color: var(--ink3);
    font-weight: 400;
  }
  .val small {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .src {
    display: block;
    margin-top: 10px;
    color: var(--ink3);
    font-size: 12px;
  }
  .pn {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 8px 24px;
    margin-top: 32px;
    padding-top: 20px;
    border-top: 1px solid var(--rule);
    font-size: 14px;
  }
</style>
