<script lang="ts">
  import { formatClock, formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { OccurrenceDto } from "../../lib/types";
  import { formatBytes, formatRate } from "./format";

  let { dto }: { dto: OccurrenceDto } = $props();

  const MAX_ROWS = 12;

  interface Row {
    pid: number;
    name: string;
    cpu: number;
    mem: number;
    io: number;
    gpu: number;
  }

  // Each program's peak across the window's rankings, heaviest on the processor first.
  const rows = $derived.by(() => {
    const peaks = new Map<number, Row>();
    for (const p of dto.window.tops.flatMap((top) => top.entries)) {
      const row = peaks.get(p.pid) ?? { pid: p.pid, name: p.name, cpu: 0, mem: 0, io: 0, gpu: 0 };
      row.cpu = Math.max(row.cpu, p.cpu_pct);
      row.mem = Math.max(row.mem, p.private_bytes);
      row.io = Math.max(row.io, p.read_bps + p.write_bps);
      row.gpu = Math.max(row.gpu, p.gpu_pct);
      peaks.set(p.pid, row);
    }
    return [...peaks.values()].sort((a, b) => b.cpu - a.cpu || b.mem - a.mem).slice(0, MAX_ROWS);
  });
</script>

<section class="card tech" aria-labelledby="occ-tech">
  <h2 id="occ-tech" class="sub">{t("ui.occurrence.tech.title")}</h2>
  <h3>{t("ui.occurrence.tech.processes")}</h3>
  {#if rows.length}
    <div class="scroll">
      <table>
        <caption>{t("ui.occurrence.tech.processes.caption")}</caption>
        <thead>
          <tr>
            <th scope="col">{t("ui.occurrence.tech.col.process")}</th>
            <th scope="col" class="opt">{t("ui.occurrence.tech.col.pid")}</th>
            <th scope="col">{t("ui.occurrence.tech.col.cpu")}</th>
            <th scope="col">{t("ui.occurrence.tech.col.mem")}</th>
            <th scope="col">{t("ui.occurrence.tech.col.io")}</th>
            <th scope="col" class="opt">{t("ui.occurrence.tech.col.gpu")}</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.pid)}
            <tr>
              <th scope="row" title={row.name}>{row.name}</th>
              <td class="opt">{row.pid}</td>
              <td>{formatPercent(row.cpu, 1)}</td>
              <td>{formatBytes(row.mem)}</td>
              <td>{formatRate(row.io)}</td>
              <td class="opt">{formatPercent(row.gpu)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="none">{t("ui.occurrence.tech.no_processes")}</p>
  {/if}

  <h3>{t("ui.occurrence.tech.events")}</h3>
  {#if dto.window.events.length}
    <div class="scroll">
      <table class="events">
        <thead>
          <tr>
            <th scope="col">{t("ui.occurrence.tech.col.time")}</th>
            <th scope="col" class="opt">{t("ui.occurrence.tech.col.channel")}</th>
            <th scope="col">{t("ui.occurrence.tech.col.provider")}</th>
            <th scope="col">{t("ui.occurrence.tech.col.id")}</th>
            <th scope="col">{t("ui.occurrence.tech.col.meaning")}</th>
          </tr>
        </thead>
        <tbody>
          {#each dto.window.events as event (event.record_id)}
            <tr>
              <td>{formatClock(event.ts_ms)}</td>
              <td class="opt">{event.channel}</td>
              <th scope="row" title={event.provider}>{event.provider}</th>
              <td>{event.id}</td>
              <td>{t(`ui.occurrence.event.${event.meaning}`)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="none">{t("ui.occurrence.tech.no_events")}</p>
  {/if}
</section>

<style>
  .tech {
    margin-top: 14px;
  }
  .sub {
    margin: 0 0 8px;
    font: 600 15px/1.3 var(--f);
  }
  h3 {
    margin: 14px 0 6px;
    color: var(--ink2);
    font: 600 13px/1.35 var(--f);
  }
  h2 + h3 {
    margin-top: 4px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  caption {
    caption-side: bottom;
    padding-top: 8px;
    text-align: left;
    color: var(--ink2);
    font-size: 13px;
  }
  th,
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--stroke2);
    text-align: right;
    white-space: nowrap;
  }
  thead th {
    color: var(--ink2);
    font-weight: 600;
  }
  th:first-child,
  td:first-child,
  tbody th {
    text-align: left;
  }
  tbody th {
    font-weight: 400;
  }
  .events th:last-child,
  .events td:last-child {
    text-align: left;
    white-space: normal;
  }
  .none {
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
  }
  @container content (max-width: 720px) {
    table:not(.events) {
      table-layout: fixed;
    }
    table:not(.events) tbody th {
      overflow: hidden;
      text-overflow: ellipsis;
    }
    th,
    td {
      padding: 6px 4px;
    }
    thead th {
      white-space: normal;
      vertical-align: bottom;
    }
  }
  .scroll {
    overflow-x: auto;
  }
  /* With very little room the PID, graphics and log columns give way and the table scrolls. */
  @container content (max-width: 480px) {
    .opt {
      display: none;
    }
    table {
      min-width: 400px;
    }
  }
</style>
