<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import HistList from "../../components/HistList.svelte";
  import Icon from "../../components/Icon.svelte";
  import Loading from "../../components/Loading.svelte";
  import MiniSpark from "../../components/MiniSpark.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { openStorageSense } from "../../lib/api";
  import { errorText } from "../../lib/errors";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { startup } from "../../lib/startup.svelte";
  import { toast } from "../../lib/toast.svelte";
  import type { DetailLevel, FindingDto } from "../../lib/types";
  import { formatDayShort } from "../occurrence/format";
  import Finding from "./Finding.svelte";
  import PhaseChart from "./PhaseChart.svelte";
  import {
    bootSub,
    causeText,
    ceilingOf,
    failures,
    failureSub,
    judge,
    ledeParts,
    type Phase,
    PHASES,
    phaseMs,
    rangeText,
    runText,
    series,
    stamp,
    tempIsLarge,
    tempSub,
    totalText,
    tpmText,
    valueText,
  } from "./startup";

  /** The Startup area of Health: the phases of the last startup, the startup apps, driver failures and the Temp folder. */
  let { finding = null, level }: { finding?: FindingDto | null; level: DetailLevel } = $props();

  /** The startup apps whose name is this one belong to Vestigium itself, which is switched off in Settings. */
  const OWN_NAME = "Vestigium";

  const dto = $derived(startup.dto);
  const error = $derived(startup.error);

  const boots = $derived(dto?.boots ?? []);
  const last = $derived(boots.length ? boots[boots.length - 1] : null);
  const failed = $derived(failures(boots));
  const newestFirst = $derived([...boots].reverse());
  const canDisable = $derived(dto?.startup_apps.some((a) => a.enabled && !a.protected && a.name !== OWN_NAME) ?? false);

  onMount(() => {
    void startup.load();
  });

  async function openStorage(): Promise<void> {
    try {
      await openStorageSense();
    } catch (e) {
      toast.show(errorText(e));
    }
  }

  const day = (ms: number) => formatDayShort(ms);
  const name = (p: Phase) => t(`ui.health.startup.phase.${p.id}`);
</script>

{#if dto === null && error !== null}
  <div class="gap"><ErrorView {error} heading="h2" onretry={() => startup.load()} /></div>
{:else if dto === null}
  <Loading />
{:else}
  <p class="lede">{#each ledeParts(boots) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}</p>

  <Block title={t("ui.health.startup.phases")} first>
    <RowList>
      {#each PHASES as p (p.id)}
        {#if last}
          {@const ms = phaseMs(last, p.id)}
          {@const verdict = judge(last, p.id)}
          <RowCard icon={p.icon} title={name(p)} sub={t(`ui.health.startup.phase.${p.id}.sub`)}>
            {#snippet right()}
              <span class="val">
                <b class:none={ms === null}>{valueText(ms)}</b>
                <small>{rangeText(p.id)}</small>
              </span>
              <StatusMark kind={verdict.kind} word={verdict.word} reveal />
              <span class="mini"><MiniSpark values={series(boots, p.id)} ceiling={ceilingOf(boots, p.id)} /></span>
            {/snippet}
            {#snippet expand()}
              {@render phaseBody(p, last)}
            {/snippet}
          </RowCard>
        {:else}
          <RowCard static chevron={false} icon={p.icon} title={name(p)} sub={t(`ui.health.startup.phase.${p.id}.sub`)}>
            {#snippet right()}
              <span class="val"><b class="none">{valueText(null)}</b></span>
              <StatusMark kind="info" word={t("ui.health.mark.Unknown")} reveal />
            {/snippet}
          </RowCard>
        {/if}
      {/each}
    </RowList>
  </Block>

  <Block title={t("ui.health.startup.apps")}>
    {#if dto.startup_apps.length}
      <RowList>
        {#each dto.startup_apps as entry (entry.key)}
          <RowCard
            static
            chevron={false}
            icon="app"
            title={entry.name}
            sub={t(entry.protected ? "ui.health.startup.apps.protected" : entry.enabled ? "ui.health.startup.apps.on" : "ui.health.startup.apps.off")}
          >
            {#snippet right()}
              {#if entry.name === OWN_NAME}
                <a class="link" href={href.settings("watching")}>{t("ui.health.startup.apps.monitoring")}</a>
              {:else if entry.enabled && !entry.protected}
                <a class="btn sm" href={href.tweaks(entry.key)} aria-label={t("ui.health.startup.apps.disable_named", { name: entry.name })}>{t("ui.health.startup.apps.disable")}</a>
              {/if}
            {/snippet}
          </RowCard>
        {/each}
      </RowList>
      {#if canDisable}<p class="note">{t("ui.health.startup.apps.note")}</p>{/if}
    {:else}
      <p class="empty">{t("ui.health.startup.apps.none")}</p>
    {/if}
  </Block>

  {#if last}
    <Block title={t("ui.health.startup.fail.title")}>
      {#if failed.length}
        <RowList>
          {#each failed as f (f.name)}
            {#if f.name.toLowerCase() === "dam"}
              <RowCard title={t("ui.health.startup.fail.row", { name: f.name })} sub={failureSub(f.count, boots.length)}>
                {#snippet lead()}<StatusMark kind="info" size={20} bare />{/snippet}
                {#snippet expand()}
                  <p class="p">{t("ui.health.startup.fail.dam")}</p>
                  {#if level === "Technical"}<span class="src">{t("ui.health.startup.source", { name: "Service Control Manager 7026" })}</span>{/if}
                {/snippet}
              </RowCard>
            {:else}
              <RowCard static chevron={false} title={t("ui.health.startup.fail.row", { name: f.name })} sub={failureSub(f.count, boots.length)}>
                {#snippet lead()}<StatusMark kind="info" size={20} bare />{/snippet}
              </RowCard>
            {/if}
          {/each}
        </RowList>
      {:else}
        <p class="empty">{t("ui.health.startup.fail.none")}</p>
      {/if}
    </Block>
  {/if}

  <Block title={t("ui.health.startup.actions")}>
    <RowList>
      {#if tempIsLarge(dto.temp)}
        <RowCard icon="trash" title={t("ui.health.startup.temp.title")} sub={tempSub(dto.temp)}>
          {#snippet right()}<StatusMark kind="info" word={t("ui.health.startup.temp.large")} reveal />{/snippet}
          {#snippet expand()}{@render tempBody()}{/snippet}
        </RowCard>
      {:else}
        <RowCard icon="trash" title={t("ui.health.startup.temp.title")} sub={tempSub(dto.temp)}>
          {#snippet expand()}{@render tempBody()}{/snippet}
        </RowCard>
      {/if}
    </RowList>
  </Block>

  {#if last}
    <Block title={t("ui.health.startup.boots.block")}>
      <RowList>
        <RowCard
          icon="clock"
          title={tn("ui.health.startup.boots.title", boots.length)}
          sub={t("ui.health.startup.boots.range", { from: day(boots[0].kernel_start_ms), to: day(last.kernel_start_ms) })}
        >
          {#snippet expand()}
            <div class="scroll">
              <HistList
                entries={newestFirst.map((boot) => {
                  const verdict = judge(boot, "profile");
                  return {
                    key: String(boot.kernel_start_ms),
                    date: stamp(boot.kernel_start_ms),
                    title: totalText(boot),
                    sub: bootSub(boot),
                    mark: verdict,
                    href: href.startupBoot(boot.kernel_start_ms),
                  };
                })}
              />
            </div>
          {/snippet}
        </RowCard>
      </RowList>
    </Block>
  {/if}
{/if}

{#snippet tempBody()}
  <p class="p">{t("ui.health.startup.temp.body")}</p>
  <div class="acts">
    <button type="button" class="btn sm" onclick={openStorage}><Icon name="outside" />{t("ui.health.startup.temp.open")}</button>
  </div>
{/snippet}

{#snippet phaseBody(p: Phase, boot: NonNullable<typeof last>)}
  {#if p.id === "profile" && finding}
    <Finding {finding} {level} bare />
  {:else}
    <p class="p">{causeText(boot, p.id)}</p>
    {#if p.id === "profile" && tpmText(boot)}<p class="p">{tpmText(boot)}</p>{/if}
  {/if}
  {#if p.id === "apps" && boot.apps.length}
    <h3 class="sub">{t("ui.health.startup.runs")}</h3>
    <ul class="runs">
      {#each boot.apps as run (run.start_ms + run.command)}
        <li><span>{run.command}</span><small>{runText(run, boot)}</small></li>
      {/each}
    </ul>
  {/if}
  {#if level === "Technical"}<span class="src">{t("ui.health.startup.source", { name: p.source })}</span>{/if}
  {#if boots.length > 1 && series(boots, p.id).some((v) => v !== null)}<PhaseChart {boots} phase={p} />{/if}
{/snippet}

<style>
  .gap {
    margin-top: 14px;
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
  .mini {
    display: flex;
    flex: none;
    width: 96px;
  }
  @container content (max-width: 760px) {
    .mini {
      display: none;
    }
  }
  .btn.sm {
    min-height: 30px;
    padding: 0 14px;
    font-size: 13.5px;
  }
  .btn :global(svg) {
    flex: none;
    width: 16px;
    height: 16px;
  }
  .link {
    font-size: 13.5px;
  }
  .acts {
    margin-top: 8px;
  }
  .src {
    display: block;
    margin-top: 2px;
    color: var(--ink3);
    font-size: 12px;
  }
  .sub {
    margin: 16px 0 6px;
  }
  .runs {
    display: grid;
    gap: 6px;
    max-width: 72ch;
    margin: 0 0 4px;
    padding: 0;
    font-size: 14px;
    list-style: none;
  }
  .runs li {
    display: grid;
  }
  .runs small {
    color: var(--ink2);
    font-size: 13px;
  }
  .scroll {
    max-height: 340px;
    margin: 0 -8px;
    padding: 0 8px;
    overflow: auto;
    scrollbar-gutter: stable;
  }
</style>
