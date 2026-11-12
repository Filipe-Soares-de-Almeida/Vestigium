<script lang="ts">
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { AppFrames, DetailLevel } from "../../lib/types";
  import { hitchesOften, OFTEN_PCT } from "./model";
  import { type AppLine, frameText, shareNumber } from "./words";

  let {
    rows,
    quiet,
    often,
    all,
    level,
  }: { rows: AppLine[]; quiet: string | null; often: string[]; all: AppFrames[]; level: DetailLevel } = $props();
</script>

{#if level === "Technical"}
  <table class="tbl">
    <caption class="sr">{t("ui.captures.apps.caption")}</caption>
    <thead>
      <tr>
        <th scope="col">{t("ui.captures.col.app")}</th>
        <th scope="col" class="num">PID</th>
        <th scope="col" class="num">{t("ui.captures.col.frames")}</th>
        <th scope="col" class="num">{t("ui.captures.col.median")}</th>
        <th scope="col" class="num">p99</th>
        <th scope="col" class="num">{t("ui.captures.col.worst")}</th>
        <th scope="col" class="num">{t("ui.captures.col.late")}</th>
        <th scope="col" class="num">%</th>
        <th scope="col">{t("ui.captures.apps.col.state")}</th>
      </tr>
    </thead>
    <tbody>
      {#each all as app (app.pid)}
        <tr class:often={hitchesOften(app)}>
          <td class="nm" title={app.name}>{app.name}</td>
          <td class="num">{app.pid}</td>
          <td class="num">{formatNumber(app.frames)}</td>
          <td class="num">{frameText(app.median_ms)}</td>
          <td class="num">{frameText(app.p99_ms)}</td>
          <td class="num">{frameText(app.worst_ms)}</td>
          <td class="num">{formatNumber(app.hitches)}</td>
          <td class="num">{shareNumber(app)}</td>
          <td>
            {#if hitchesOften(app)}
              <StatusMark kind="attention" word={t("ui.captures.apps.over", { limit: OFTEN_PCT })} />
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="note">{t("ui.captures.apps.foot.t")}</p>
{:else}
  {#if rows.length}
    <RowList>
      {#each rows as row (row.key)}
        <RowCard
          static
          chevron={false}
          icon="app"
          title={level === "Detailed" ? `${row.name} · ${row.exe}` : row.name}
          sub={row.detail ? `${row.summary} ${row.detail}` : row.summary}
        >
          {#snippet right()}
            <span class="share">{row.share}</span>
            {#if row.often}
              <StatusMark kind="attention" word={t("ui.captures.apps.over", { limit: OFTEN_PCT })} />
            {/if}
          {/snippet}
        </RowCard>
      {/each}
    </RowList>
  {:else}
    <p class="empty">{t("ui.captures.apps.none")}</p>
  {/if}
  {#if quiet}
    <p class="note">{quiet}</p>
  {/if}
{/if}
{#each often as line, i (i)}
  <p class="note">{line}</p>
{/each}

<style>
  .share {
    color: var(--ink2);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
</style>
