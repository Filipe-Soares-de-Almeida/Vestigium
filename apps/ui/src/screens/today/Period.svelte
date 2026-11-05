<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import Steps from "../../components/Steps.svelte";
  import { episodeDetail } from "../../lib/api";
  import { formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel, Episode, EpisodeDetailDto } from "../../lib/types";
  import WindowChart from "../occurrence/WindowChart.svelte";
  import { periodPrograms, periodStats, periodSteps, periodView } from "./period";
  import { occurrenceTitle } from "./text";

  let {
    episode,
    byTest,
    level,
    id,
    label,
  }: { episode: Episode; byTest: boolean; level: DetailLevel; id: string; label: string } = $props();

  let dto = $state.raw<EpisodeDetailDto | null>(null);
  let error = $state<unknown>(null);

  const key = $derived(`${episode.start_ms}-${episode.end_ms}`);
  const view = $derived(dto ? periodView(episode, dto, level) : null);
  const stats = $derived(dto ? periodStats(episode, dto) : []);
  const programs = $derived(dto ? periodPrograms(episode, dto) : null);
  const steps = $derived(periodSteps(episode, byTest));

  async function load(): Promise<void> {
    error = null;
    try {
      dto = await episodeDetail(episode);
    } catch (err) {
      error = err;
    }
  }

  // Reads the detail once per episode; the day's refresh hands over an equal one.
  $effect(() => {
    void key;
    untrack(() => void load());
  });
</script>

<div class="body" {id} role="region" aria-label={label}>
  {#if error !== null}
    <ErrorView {error} onretry={load} />
  {:else if !dto || !view || !programs}
    <Loading />
  {:else}
    <dl class="stats">
      {#each stats as stat (stat.label)}
        <div>
          <dt>{stat.label}</dt>
          <dd>{stat.value}</dd>
        </div>
      {/each}
    </dl>
    {#if view.lanes.length}
      <WindowChart
        {view}
        clock
        label={t("ui.today.period.chart.label")}
        hint={t("ui.today.period.chart.hint")}
        keys={t("ui.today.period.chart.keys")}
      />
    {:else}
      <p class="muted">{t("ui.today.period.chart.none")}</p>
    {/if}

    <h4>{t("ui.today.period.programs")}</h4>
    {#if programs.rows.length}
      <table>
        <thead>
          <tr>
            <th scope="col">{t("ui.today.period.col.program")}</th>
            <th scope="col" class="n">{programs.columns[0]}</th>
            <th scope="col" class="n">{programs.columns[1]}</th>
          </tr>
        </thead>
        <tbody>
          {#each programs.rows as row (row.pid)}
            <tr>
              <th scope="row" title={row.name}>{row.name}</th>
              <td class="n">{row.values[0]}</td>
              <td class="n">{row.values[1]}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="muted">{t("ui.today.period.programs.none")}</p>
    {/if}

    {#if dto.occurrences.length}
      <h4>{t("ui.today.period.freezes")}</h4>
      <ul class="occ">
        {#each dto.occurrences as o (o.id)}
          <li>
            <span class="dot" aria-hidden="true"></span>
            <a href={href.occurrence(o.id)}>{formatTime(o.start_ms)} · {occurrenceTitle(o, level)}</a>
          </li>
        {/each}
      </ul>
    {/if}

    <h4>{t("ui.today.period.steps")}</h4>
    <Steps {steps} />
  {/if}
</div>

<style>
  .body {
    padding: 14px 14px 16px;
    border-top: 1px solid var(--stroke2);
    animation: rise 0.22s cubic-bezier(0.2, 0.9, 0.3, 1);
  }
  @keyframes rise {
    from {
      transform: translateY(-4px);
    }
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 28px;
    margin: 0 0 14px;
  }
  .stats div {
    display: grid;
    gap: 2px;
  }
  dt {
    color: var(--ink2);
    font-size: 12.5px;
  }
  dd {
    margin: 0;
    font-weight: 600;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }
  h4 {
    margin: 16px 0 6px;
    color: var(--ink2);
    font: 600 13px/1.3 var(--f);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    font-size: 13.5px;
  }
  th,
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--stroke2);
    text-align: left;
  }
  thead th {
    color: var(--ink2);
    font-weight: 600;
    font-size: 12.5px;
  }
  tbody th {
    overflow: hidden;
    font-weight: 400;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    width: 30%;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .occ {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .occ li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--freeze);
  }
  .occ a {
    font-weight: 600;
    text-decoration: none;
  }
  .occ a:hover {
    text-decoration: underline;
  }
  .muted {
    margin: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .body {
      animation: none;
    }
  }
  @media (forced-colors: active) {
    .dot {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
