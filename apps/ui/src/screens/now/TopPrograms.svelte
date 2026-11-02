<script lang="ts">
  import { formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { ProcessTop } from "../../lib/types";
  import { formatBytes, formatRate } from "./lanes";

  let { top }: { top: ProcessTop | null } = $props();

  // The watcher ranks the busiest programs by processor, each summing its processes; five show.
  const rows = $derived(top?.entries.slice(0, 5) ?? []);
</script>

<section class="card top" aria-labelledby="top-title">
  <h2 id="top-title">{t("ui.now.top.title")}</h2>
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
            <th scope="row" class="fill" title={row.name}>{row.name}</th>
            <td data-label={t("ui.now.top.cpu")}>{formatPercent(row.cpu_pct, 1)}</td>
            <td data-label={t("ui.now.top.memory")}>{formatBytes(row.private_bytes)}</td>
            <td data-label={t("ui.now.top.disk")}>{formatRate(row.read_bps + row.write_bps)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .top {
    container: top / inline-size;
  }
  h2 {
    font: 600 15px/1.3 var(--f);
    margin: 0 0 8px;
  }
  p {
    margin: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  th,
  td {
    padding: 6px 8px;
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
    font-weight: 600;
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
