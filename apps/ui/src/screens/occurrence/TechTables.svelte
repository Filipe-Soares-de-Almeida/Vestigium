<script lang="ts">
  import { here, href } from "../../lib/router.svelte";
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
  }

  // Each program's peak across the window's rankings, heaviest on the processor first.
  const rows = $derived.by(() => {
    const peaks = new Map<number, Row>();
    for (const p of dto.window.tops.flatMap((top) => top.entries)) {
      const row = peaks.get(p.pid) ?? { pid: p.pid, name: p.name, cpu: 0, mem: 0, io: 0 };
      row.cpu = Math.max(row.cpu, p.cpu_pct);
      row.mem = Math.max(row.mem, p.private_bytes);
      row.io = Math.max(row.io, p.read_bps + p.write_bps);
      peaks.set(p.pid, row);
    }
    return [...peaks.values()].sort((a, b) => b.cpu - a.cpu || b.mem - a.mem).slice(0, MAX_ROWS);
  });
</script>

<div class="tech">
  <h3 class="h3">{t("ui.occurrence.tech.processes")}</h3>
  {#if rows.length}
    <table class="tbl">
      <thead>
        <tr>
          <th scope="col">{t("ui.occurrence.tech.col.process")}</th>
          <th scope="col" class="num">{t("ui.occurrence.tech.col.cpu")}</th>
          <th scope="col" class="num">{t("ui.occurrence.tech.col.mem")}</th>
          <th scope="col" class="num">{t("ui.occurrence.tech.col.io")}</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.pid)}
          <tr>
            <td class="nm" title={row.name}>
              <a class="row-a" href={href.detail("process", [row.name, dto.id, row.pid], { from: here(), label: t("ui.occurrence.crumb") })}>{row.name}</a>
            </td>
            <td class="num">{formatPercent(row.cpu, 1)}</td>
            <td class="num">{formatBytes(row.mem)}</td>
            <td class="num">{formatRate(row.io)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="empty">{t("ui.occurrence.tech.no_processes")}</p>
  {/if}

  <h3 class="h3">{t("ui.occurrence.tech.events")}</h3>
  {#if dto.window.events.length}
    <table class="tbl">
      <thead>
        <tr>
          <th scope="col">{t("ui.occurrence.tech.col.time")}</th>
          <th scope="col">{t("ui.occurrence.tech.col.provider")}</th>
          <th scope="col">{t("ui.occurrence.tech.col.id")}</th>
          <th scope="col">{t("ui.occurrence.tech.col.meaning")}</th>
        </tr>
      </thead>
      <tbody>
        {#each dto.window.events as event (event.record_id)}
          {@const meaning = t(`ui.occurrence.event.${event.meaning}`)}
          <tr>
            <td>{formatClock(event.ts_ms)}</td>
            <td>{event.provider}</td>
            <td>{event.id}</td>
            <td class="nm" title={meaning}>
              <a class="row-a" href={href.detail("event", [dto.id, event.record_id], { from: here(), label: t("ui.occurrence.crumb") })}>{meaning}</a>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="empty">{t("ui.occurrence.tech.no_events")}</p>
  {/if}
</div>

<style>
  .tech > .h3:first-child {
    margin-top: 0;
  }
  .h3 {
    margin: 20px 0 8px;
  }
</style>
