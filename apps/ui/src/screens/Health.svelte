<script lang="ts">
  import ConcernMark from "../components/ConcernMark.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import Segmented from "../components/Segmented.svelte";
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { health } from "../lib/health.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { DetailLevel } from "../lib/types";
  import { AREAS, areaState, findArea } from "./health/areas";
  import Finding from "./health/Finding.svelte";
  import Scan from "./health/Scan.svelte";
  import { checkText, findingTitle, lede, when } from "./health/words";

  let { area: areaId }: { area: string | null } = $props();

  const LEVELS: DetailLevel[] = ["Simple", "Detailed", "Technical"];

  /** The level picked here, until the saved setting catches up with it. */
  let picked = $state<DetailLevel | null>(null);

  const level = $derived(picked ?? app.settings?.settings.detail_level ?? "Simple");
  const area = $derived(findArea(areaId));
  const states = $derived(health.dto ? AREAS.map((a) => areaState(a, health.dto!)) : []);
  const current = $derived(area && health.dto ? areaState(area, health.dto) : null);
  const checks = $derived(states.reduce((n, s) => n + s.checks.filter((c) => c.mark === "Ok").length, 0));

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

  function retry(): void {
    health.error = null;
    void health.load();
  }
</script>

{#snippet levels()}
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
  </div>
{/snippet}

{#if area}
  <nav class="crumb" aria-label={t("ui.occurrence.path")}>
    <a href={href.health()}>{t("ui.health.title")}</a>
    <Icon name="chevron" />
    <span aria-current="page">{t(`ui.health.area.${area.id}`)}</span>
  </nav>
  <h1 class="sr">{t(`ui.health.area.${area.id}`)}</h1>
{:else}
  <h1>{t("ui.health.title")}</h1>
{/if}

{#if health.dto === null && health.error !== null}
  <div class="gap"><ErrorView error={health.error} heading="h2" onretry={retry} /></div>
{:else if health.dto === null}
  <Loading />
{:else if current}
  {@render levels()}
  {#each current.open as finding (finding.id)}
    <Finding {finding} {level} />
  {/each}
  {#if current.checks.some((c) => c.mark === "Unknown")}
    <Scan {level} />
  {/if}
  <section class="card list" aria-labelledby="ok-title">
    <h2 id="ok-title">{t("ui.health.area.ok_title")}</h2>
    {#each current.checks as check (check.kind)}
      <p class="row"><ConcernMark mark={check.mark} word={false} /><span>{checkText(check)}</span></p>
    {:else}
      <p class="muted empty">{t("ui.health.area.ok_none")}</p>
    {/each}
  </section>
  {#if current.closed.length}
    <section class="card list" aria-labelledby="closed-title">
      <h2 id="closed-title">{t("ui.health.area.closed_title")}</h2>
      {#each current.closed as finding (finding.id)}
        <p class="row">
          <ConcernMark mark="Closed" word={false} />
          <span>
            {findingTitle(finding, level)}
            <small>
              {t("ui.health.closed_line", { opened: when(finding.opened_ms), closed: when(finding.closed_ms ?? 0) })}
            </small>
          </span>
        </p>
      {/each}
    </section>
  {/if}
{:else}
  <p class="lede">
    {#if app.history === "empty"}
      {t("ui.health.lede.first")}
    {:else}
      {#each lede(health.dto, level, checks) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
    {/if}
  </p>
  <nav class="card areas" aria-label={t("ui.health.areas_label")}>
    {#each states as s (s.area.id)}
      {@const tone = s.mark === "Critical" ? "freeze" : s.mark === "Attention" ? "press" : null}
      <a class="area" href={href.health(s.area.id)}>
        <span
          class="icochip"
          style:background={tone ? `var(--${tone}-t)` : "var(--card2)"}
          style:color={tone ? `var(--${tone})` : "var(--ink2)"}
        >
          <Icon name={s.area.icon} />
        </span>
        <span class="what">
          <b>{t(`ui.health.area.${s.area.id}`)}</b>
          <small>
            {#if s.open.length > 1}
              {t("ui.health.area.more", { title: findingTitle(s.open[0], level), count: s.open.length - 1 })}
            {:else if s.open.length}
              {findingTitle(s.open[0], level)}
            {:else if s.mark === "Unknown"}
              {t("ui.health.area.unknown")}
            {:else}
              {tn("ui.health.area.fine", s.checks.length)}
            {/if}
          </small>
        </span>
        <span class="side">
          <ConcernMark mark={s.mark} />
          {#if s.open.length}<span>{tn("ui.health.area.points", s.open.length)}</span>{/if}
        </span>
        <Icon name="chevron" />
      </a>
    {/each}
  </nav>
  <Scan {level} />
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
  .tools {
    margin-top: 14px;
  }
  .gap {
    margin-top: 14px;
  }
  .areas {
    display: grid;
    padding: 6px;
  }
  .area {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto 12px;
    gap: 14px;
    align-items: center;
    padding: 12px 10px;
    border-radius: 8px;
    color: inherit;
    text-decoration: none;
  }
  .area:hover {
    background: var(--card2);
  }
  .area + .area {
    border-top: 1px solid var(--stroke2);
    border-radius: 0;
  }
  .area + .area:hover {
    border-radius: 8px;
  }
  .what b {
    display: block;
    font-weight: 600;
  }
  .what small {
    display: block;
    margin-top: 2px;
    overflow: hidden;
    color: var(--ink2);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .side {
    display: grid;
    justify-items: end;
    gap: 2px;
    color: var(--ink2);
    font-size: 13px;
  }
  .area > :global(svg:last-child) {
    color: var(--ink3);
  }
  .list {
    margin-top: 14px;
    padding: 10px;
  }
  .list h2 {
    margin: 0;
    padding: 6px 10px 4px;
    font: 600 15px/1.3 var(--f);
  }
  .row {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    margin: 0;
    padding: 8px 10px;
    font-size: 14px;
    line-height: 1.45;
  }
  .row + .row {
    border-top: 1px solid var(--stroke2);
  }
  .row :global(.cn) {
    padding-top: 2px;
  }
  .row small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .empty {
    margin: 4px 10px 8px;
    font-size: 14px;
  }
  @container content (max-width: 560px) {
    .area {
      grid-template-columns: 40px minmax(0, 1fr) 12px;
    }
    .side {
      grid-column: 2;
      grid-row: 2;
      justify-items: start;
    }
  }
</style>
