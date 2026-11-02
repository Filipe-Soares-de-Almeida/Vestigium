<script lang="ts">
  import Icon from "../../components/Icon.svelte";
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
  <div class="scroll">
    <table>
      <caption class="sr">{t("ui.captures.apps.caption")}</caption>
      <thead>
        <tr>
          <th scope="col">{t("ui.captures.col.app")}</th>
          <th scope="col" class="n">PID</th>
          <th scope="col" class="n">{t("ui.captures.col.frames")}</th>
          <th scope="col" class="n">{t("ui.captures.col.median")}</th>
          <th scope="col" class="n">p99</th>
          <th scope="col" class="n">{t("ui.captures.col.worst")}</th>
          <th scope="col" class="n">{t("ui.captures.col.late")}</th>
        </tr>
      </thead>
      <tbody>
        {#each all as app (app.pid)}
          <tr>
            <td class="fill" title={app.name}>{app.name}</td>
            <td class="n">{app.pid}</td>
            <td class="n">{formatNumber(app.frames)}</td>
            <td class="n">{frameText(app.median_ms)}</td>
            <td class="n">{frameText(app.p99_ms)}</td>
            <td class="n">{frameText(app.worst_ms)}</td>
            <td class="n">{formatNumber(app.hitches)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <p class="foot">{t("ui.captures.apps.foot.t")}</p>
{:else}
  {#each rows as row (row.key)}
    <div class="app">
      <span class="icochip"><Icon name="app" /></span>
      <div>
        <b>{row.name}{#if level === "Detailed"} <span class="exe">· {row.exe}</span>{/if}</b>
        <small>{row.summary}</small>
        {#if row.detail}<small class="detail">{row.detail}</small>{/if}
      </div>
    </div>
  {:else}
    <p class="none">{t("ui.captures.apps.none")}</p>
  {/each}
  {#if quiet}
    <p class="foot"><span class="state-dot" style:background="var(--calm)"></span> {quiet}</p>
  {/if}
{/if}

<style>
  .app {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 12px;
    align-items: start;
    padding: 4px 0;
  }
  .app + .app {
    margin-top: 6px;
    padding-top: 10px;
    border-top: 1px solid var(--stroke2);
  }
  .icochip {
    background: var(--busy-t);
    color: var(--busy);
  }
  b {
    display: block;
    font-weight: 600;
  }
  .exe {
    margin-left: 4px;
    color: var(--ink2);
    font-weight: 400;
  }
  small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  .detail {
    margin-top: 6px;
    color: var(--ink);
  }
  .none {
    margin: 0;
    color: var(--ink2);
  }
  .foot {
    margin: 10px 0 0;
    max-width: 80ch;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  .scroll {
    overflow-x: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  th {
    padding: 0 8px 6px;
    color: var(--ink2);
    font: 600 12.5px/1.35 var(--f);
    text-align: left;
    white-space: nowrap;
  }
  td {
    padding: 9px 8px;
    border-top: 1px solid var(--stroke2);
  }
  th:first-child,
  td:first-child {
    padding-left: 0;
  }
  th:last-child,
  td:last-child {
    padding-right: 0;
  }
  .n {
    text-align: right;
    white-space: nowrap;
  }
  .fill {
    font-weight: 600;
  }
</style>
