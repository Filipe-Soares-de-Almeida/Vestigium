<script lang="ts">
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { AppFrames, DetailLevel } from "../../lib/types";
  import { type AppLine, frameText } from "./words";

  let {
    rows,
    quiet,
    all,
    level,
  }: { rows: AppLine[]; quiet: string | null; all: AppFrames[]; level: DetailLevel } = $props();
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
      </tr>
    </thead>
    <tbody>
      {#each all as app (app.pid)}
        <tr>
          <td class="nm" title={app.name}>{app.name}</td>
          <td class="num">{app.pid}</td>
          <td class="num">{formatNumber(app.frames)}</td>
          <td class="num">{frameText(app.median_ms)}</td>
          <td class="num">{frameText(app.p99_ms)}</td>
          <td class="num">{frameText(app.worst_ms)}</td>
          <td class="num">{formatNumber(app.hitches)}</td>
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
        />
      {/each}
    </RowList>
  {:else}
    <p class="empty">{t("ui.captures.apps.none")}</p>
  {/if}
  {#if quiet}
    <p class="note">{quiet}</p>
  {/if}
{/if}
