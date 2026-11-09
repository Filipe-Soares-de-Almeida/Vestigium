<script lang="ts">
  import { untrack } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import RowList from "../../components/RowList.svelte";
  import Steps from "../../components/Steps.svelte";
  import { episodeDetail, marks, timeline } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel, Episode, EpisodeDetailDto } from "../../lib/types";
  import { buildDay, type Day, dayStart } from "./day";
  import { periodPrograms, periodStats, periodSteps } from "./period";
  import OccurrenceRow from "./OccurrenceRow.svelte";
  import States from "./States.svelte";

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

  /** The day chart opens on 100 minutes around the start of the period. */
  const AROUND_MS = 100 * 60_000;

  let dto = $state.raw<EpisodeDetailDto | null>(null);
  let day = $state.raw<Day | null>(null);
  let error = $state<unknown>(null);

  const key = $derived(`${episode.start_ms}-${episode.end_ms}`);
  const focus = $derived({ from: episode.start_ms - AROUND_MS / 2, to: episode.start_ms + AROUND_MS / 2 });
  const stats = $derived(dto ? periodStats(episode, dto) : []);
  const programs = $derived(dto ? periodPrograms(episode, dto) : null);
  const steps = $derived(periodSteps(episode, byTest));

  async function load(): Promise<void> {
    error = null;
    try {
      const end = Math.min(dayStart(episode.start_ms, 1) - 1, Date.now());
      const [detail, line, found] = await Promise.all([
        episodeDetail(episode),
        timeline(dayStart(episode.start_ms), end, 24 * 60),
        marks(dayStart(episode.start_ms), end),
      ]);
      dto = detail;
      day = buildDay(end, line, found);
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
  {:else if !dto || !day || !programs}
    <Loading />
  {:else}
    <Block title={t("ui.period.numbers")} first>
      <dl class="figs">
        {#each stats as stat (stat.label)}
          <div>
            <dt>{stat.label}</dt>
            <dd>{stat.value}</dd>
          </div>
        {/each}
      </dl>
    </Block>
    <States {day} {level} title={t("ui.period.day")} row={26} {focus} />
    <div class="cols2">
      <Block title={t("ui.today.period.programs")}>
        {#if programs.rows.length}
          <table class="tbl">
            <tbody>
              {#each programs.rows as row (row.pid)}
                <tr>
                  <td class="nm" title={row.name}>{row.name}</td>
                  <td class="num">{row.values[0]}</td>
                  <td class="num">{row.values[1]}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="empty">{t("ui.today.period.programs.none")}</p>
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
            <p class="empty">{t("ui.period.freezes.none")}</p>
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
  .figs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 16px 32px;
    margin: 0;
  }
  dt {
    color: var(--ink2);
    font-size: 13px;
  }
  dd {
    margin: 2px 0 0;
    font: 350 24px/1.2 var(--fd);
  }
</style>
