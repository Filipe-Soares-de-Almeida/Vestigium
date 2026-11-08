<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { here, href } from "../../lib/router.svelte";
  import type { ProcessTop } from "../../lib/types";
  import { formatBytes, formatRate } from "./lanes";

  let { top, total }: { top: ProcessTop | null; total: number } = $props();

  // The watcher ranks the busiest programs by processor, each summing its processes; five show.
  const rows = $derived(top?.entries.slice(0, 5) ?? []);
</script>

<Block title={t("ui.now.top.title")}>
  <div class="top">
    {#if rows.length === 0}
      <p class="muted">{t("ui.now.top.waiting")}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th scope="col">{t("ui.now.top.program")}</th>
            <th scope="col">{t("ui.now.top.cpu")}</th>
            <th scope="col">{t("ui.now.top.memory")}</th>
            <th scope="col">{t("ui.now.top.disk")}</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.pid)}
            <tr>
              <th scope="row" class="fill" title={row.name}><a href={href.detail("process", row.name, { from: here(), label: t("ui.item.live") })}>{row.name}</a></th>
              <td data-label={t("ui.now.top.cpu")}>{formatPercent(row.cpu_pct, 1)}</td>
              <td data-label={t("ui.now.top.memory")}>{formatBytes(row.private_bytes)}</td>
              <td data-label={t("ui.now.top.disk")}>{formatRate(row.read_bps + row.write_bps)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if total > 0}
      <p class="more"><a href={href.programs()}>{t("ui.now.top.all", { count: total })}</a></p>
    {/if}
  </div>
</Block>

<style>
  .top {
    container: top / inline-size;
  }
  p {
    margin: 0;
  }
  .more {
    margin-top: 12px;
    font-size: 14px;
  }
  .more a {
    font-weight: 600;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
    font-variant-numeric: tabular-nums;
  }
  th,
  td {
    padding: 8px;
    border-bottom: 1px solid var(--stroke2);
    text-align: right;
    white-space: nowrap;
  }
  tr > :first-child {
    padding-left: 0;
    text-align: left;
  }
  tr > :last-child {
    padding-right: 0;
  }
  thead th {
    color: var(--ink2);
    font-weight: 600;
  }
  tbody th {
    max-width: 0;
    overflow: hidden;
    font-weight: 400;
    text-overflow: ellipsis;
  }
  tbody tr:last-child > * {
    border-bottom: 0;
  }
  /* Too narrow for four columns: each program becomes its name over its labelled values. */
  @container top (max-width: 380px) {
    thead {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip-path: inset(50%);
    }
    tbody tr {
      display: flex;
      flex-wrap: wrap;
      gap: 2px 12px;
      padding: 6px 0;
      border-bottom: 1px solid var(--stroke2);
    }
    tbody tr:last-child {
      border-bottom: 0;
    }
    tbody th,
    tbody td {
      padding: 0;
      border: 0;
      text-align: left;
    }
    tbody th {
      flex-basis: 100%;
      max-width: none;
    }
    tbody td::before {
      content: attr(data-label) " ";
      color: var(--ink2);
    }
  }
</style>
