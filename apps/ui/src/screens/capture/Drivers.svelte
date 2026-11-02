<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel } from "../../lib/types";
  import type { DriverTable } from "./words";

  let { table, level }: { table: DriverTable; level: DetailLevel } = $props();
</script>

<div class="scroll">
  <table>
    <caption class="sr">{t("ui.captures.drivers.caption")}</caption>
    <thead>
      <tr>
        <th scope="col">{t(level === "Technical" ? "ui.captures.col.module" : "ui.captures.col.driver")}</th>
        {#if level === "Technical"}
          <th scope="col" class="n">DPC</th>
          <th scope="col" class="n">ISR</th>
        {/if}
        <th scope="col" class={level === "Technical" ? "n" : ""}>{t("ui.captures.col.longest")}</th>
        {#if level !== "Simple"}
          <th scope="col" class="n">{t("ui.captures.col.over")}</th>
        {/if}
        {#if level === "Technical"}
          <th scope="col" class="n">{t("ui.captures.col.total")}</th>
        {/if}
      </tr>
    </thead>
    <tbody>
      {#each table.rows as row (row.key)}
        <tr class:kernel={row.kernel}>
          <td class="who">
            <b>
              {row.name}
              {#if row.suspect}
                <span class="pill"><span class="state-dot" style:background="var(--freeze)"></span>{t("ui.captures.pill.suspect")}</span>
              {/if}
              {#if row.intermediary}
                <span class="pill">{t(level === "Technical" ? "ui.captures.pill.layer.t" : "ui.captures.pill.layer")}</span>
              {/if}
              {#if row.kernel}
                <span class="pill">{t("ui.captures.pill.kernel")}</span>
              {/if}
            </b>
            <small>{row.sub}</small>
          </td>
          {#if level === "Technical"}
            <td class="n">{row.dpcCalls}</td>
            <td class="n">{row.isrCalls}</td>
            <td class="n">{row.longest} {row.longestKind}</td>
          {:else}
            <td>
              <div class="bar">
                <span class="track" aria-hidden="true"><i class:suspect={row.suspect} style:width="{row.bar}%"></i></span>
                <b>{row.longest}</b>
              </div>
            </td>
          {/if}
          {#if level !== "Simple"}
            <td class="n">{row.over}</td>
          {/if}
          {#if level === "Technical"}
            <td class="n">{row.total}</td>
          {/if}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
{#if table.more}
  <p class="foot">{table.more}</p>
{/if}
{#if table.layers}
  <p class="foot">{table.layers}</p>
{/if}
<p class="foot">{t(level === "Simple" ? "ui.captures.drivers.foot" : "ui.captures.drivers.foot.d")}</p>

<style>
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
    vertical-align: bottom;
    white-space: nowrap;
  }
  td {
    padding: 9px 8px;
    border-top: 1px solid var(--stroke2);
    vertical-align: middle;
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
  .who {
    min-width: 200px;
    overflow-wrap: anywhere;
  }
  .who b {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
    font: 600 14px/1.35 var(--f);
  }
  .who small {
    display: block;
    margin-top: 2px;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.4;
  }
  .kernel td,
  .kernel small {
    color: var(--ink3);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 170px;
  }
  .track {
    display: block;
    flex: 1;
    min-width: 40px;
  }
  .track i {
    display: block;
    height: 8px;
    border-radius: 6px;
    background: var(--calm);
  }
  .track i.suspect {
    background: var(--freeze);
  }
  .bar b {
    min-width: 58px;
    font-size: 13px;
    text-align: right;
    white-space: nowrap;
  }
  .foot {
    margin: 10px 0 0;
    max-width: 80ch;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  @media (forced-colors: active) {
    .track i {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .track i.suspect {
      background: Highlight;
    }
  }
</style>
