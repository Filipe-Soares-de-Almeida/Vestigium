<script lang="ts">
  import { untrack } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import RowList from "../../components/RowList.svelte";
  import Steps from "../../components/Steps.svelte";
  import { episodeDetail } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel, Episode, EpisodeDetailDto } from "../../lib/types";
  import WindowChart from "../occurrence/WindowChart.svelte";
  import { periodPrograms, periodStats, periodSteps, periodView } from "./period";
  import OccurrenceRow from "./OccurrenceRow.svelte";

  let {
    episode,
    byTest,
    level,
    id,
    label,
    fromDay = null,
  }: {
    episode: Episode;
    byTest: boolean;
    level: DetailLevel;
    id: string;
    label: string;
    fromDay?: string | null;
  } = $props();

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
    <Block title={t("ui.period.numbers")} first>
      <dl class="stats">
        {#each stats as stat (stat.label)}
          <div>
            <dt>{stat.label}</dt>
            <dd>{stat.value}</dd>
          </div>
        {/each}
      </dl>
    </Block>
    <Block title={t("ui.period.chart")}>
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
    </Block>
    <div class="cols">
      <Block title={t("ui.today.period.programs")}>
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
      </Block>
      <div>
        <Block title={t("ui.today.period.freezes")}>
          {#if dto.occurrences.length}
            <RowList>
              {#each dto.occurrences as o (o.id)}
                <li><OccurrenceRow {o} {level} {fromDay} /></li>
              {/each}
            </RowList>
          {:else}
            <p class="muted">{t("ui.period.freezes.none")}</p>
          {/if}
        </Block>
        <Block title={t("ui.today.period.steps")}>
          <Steps {steps} />
        </Block>
      </div>
    </div>
  {/if}
</div>

<style>
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 40px;
    margin: 0;
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
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0 56px;
  }
  @container content (min-width: 1000px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      align-items: start;
    }
  }
  table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    font-size: 13.5px;
  }
  th,
  td {
    padding: 8px;
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
  .muted {
    margin: 0;
  }
</style>
