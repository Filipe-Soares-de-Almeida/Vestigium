<script lang="ts">
  import StatusMark from "../../components/StatusMark.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { here, href } from "../../lib/router.svelte";
  import type { DetailLevel } from "../../lib/types";
  import type { DriverTable } from "./words";

  let { table, level, captureId }: { table: DriverTable; level: DetailLevel; captureId: number } = $props();
</script>

{#if level !== "Technical"}
  <div class="drvs">
    <div class="drv drv-h" aria-hidden="true">
      <span>{t("ui.captures.col.driver")}</span>
      <span></span>
      <span>{t("ui.captures.col.longest_limit")}</span>
    </div>
    {#each table.rows as row (row.key)}
      <a class="drv" href={href.detail("driver", [captureId, row.module], { from: here(), label: t("ui.captures.crumb") })} data-reveal>
        <span class="drv-m">
          <b>{row.name}</b>
          <code>{row.module}</code>
          <small>{row.sub}</small>
        </span>
        <span class="drv-tag">
          {#if row.suspect}
            <StatusMark kind="critical" word={t("ui.captures.pill.suspect")} />
          {:else if row.intermediary}
            <span class="tag">{t("ui.captures.pill.layer")}</span>
          {:else if row.kernel}
            <span class="tag">{t("ui.captures.pill.kernel")}</span>
          {/if}
        </span>
        <span class="db" role="img" aria-label={t("ui.captures.drivers.bar", { max: row.longest })}>
          <span class="db-t"><i class="db-v" class:over={row.ratio !== null} style:width="{row.bar}%"></i><i class="db-lim" style:left="{row.limit}%"></i></span>
          <span class="db-n">{row.longest}<small>{row.ratio ? t("ui.captures.drivers.times", { times: row.ratio }) : t("ui.captures.drivers.inside")}</small></span>
        </span>
      </a>
    {/each}
  </div>
{:else}
  <table class="tbl">
    <caption class="sr">{t("ui.captures.drivers.caption")}</caption>
    <thead>
      <tr>
        <th scope="col">{t("ui.captures.col.module")}</th>
        <th scope="col" class="num">DPC</th>
        <th scope="col" class="num">ISR</th>
        <th scope="col" class="num">{t("ui.captures.col.longest")}</th>
        <th scope="col" class="num">{t("ui.captures.col.over")}</th>
        <th scope="col" class="num">{t("ui.captures.col.total")}</th>
      </tr>
    </thead>
    <tbody>
      {#each table.rows as row (row.key)}
        <tr class:kernel={row.kernel}>
          <td class="nm" title={row.sub}>
            <a class="row-a" href={href.detail("driver", [captureId, row.module], { from: here(), label: t("ui.captures.crumb") })}>{row.name}</a>
            {#if row.suspect}· {t("ui.captures.pill.suspect")}{/if}
            {#if row.intermediary}· {t("ui.captures.pill.layer.t")}{/if}
          </td>
          <td class="num">{row.dpcCalls}</td>
          <td class="num">{row.isrCalls}</td>
          <td class="num">{row.longest} {row.longestKind}</td>
          <td class="num">{row.over}</td>
          <td class="num">{row.total}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
{#if table.more}
  <p class="foot">{table.more}</p>
{/if}
{#if table.layers}
  <p class="foot">{table.layers}</p>
{/if}
<p class="foot">{t(level === "Simple" ? "ui.captures.drivers.foot" : "ui.captures.drivers.foot.d")}</p>

<style>
  .kernel td {
    color: var(--ink3);
  }
  .drvs {
    display: grid;
  }
  .drv {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) 170px minmax(220px, 1fr);
    gap: 6px 24px;
    align-items: center;
    margin: 0 -12px;
    padding: 12px;
    border-top: 1px solid var(--stroke2);
    border-radius: 6px;
    color: var(--ink);
    text-decoration: none;
  }
  a.drv:hover {
    background: var(--row-hover);
    text-decoration: none;
  }
  a.drv:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  .drv-h {
    padding-top: 0;
    padding-bottom: 8px;
    border-top: 0;
    color: var(--ink2);
    font-size: 12.5px;
    font-weight: 600;
  }
  .drv-m {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .drv-m b {
    font-size: 14px;
    font-weight: 600;
  }
  .drv-m code {
    color: var(--ink2);
    font: 12.5px var(--mono);
  }
  .drv-m small {
    color: var(--ink2);
    font-size: 13px;
  }
  .tag {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--ink2);
    font-size: 12px;
    white-space: nowrap;
  }
  .db {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .db-t {
    position: relative;
    flex: 1;
    height: 8px;
    border-radius: 4px;
    background: var(--hover);
  }
  .db-v {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    border-radius: 4px;
    background: var(--calm);
  }
  .db-v.over {
    background: var(--press);
  }
  .db-lim {
    position: absolute;
    top: -5px;
    bottom: -5px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--ink2);
  }
  .db-n {
    display: grid;
    min-width: 112px;
    font-size: 13.5px;
    text-align: right;
  }
  .db-n small {
    color: var(--ink2);
    font-size: 12px;
  }
  @container content (max-width: 760px) {
    .drv {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .drv .db {
      grid-column: 1 / -1;
    }
    .drv-h span:last-child {
      display: none;
    }
  }
  .foot {
    margin: 10px 0 0;
    max-width: 80ch;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  @media (forced-colors: active) {
    .db-v,
    .db-lim {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
