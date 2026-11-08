<script lang="ts">
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import TextField from "../components/TextField.svelte";
  import { app } from "../lib/app.svelte";
  import { formatPercent } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { here, href } from "../lib/router.svelte";
  import type { ProcessSample } from "../lib/types";
  import { formatBytes, formatRate } from "./now/lanes";
  import { NowStream } from "./now/stream.svelte";

  /** Rows the table shows at most, after the search and the sort. */
  const SHOWN = 100;
  type Key = "name" | "cpu" | "memory" | "disk";
  const VALUE: Record<Key, (p: ProcessSample) => number | string> = {
    name: (p) => p.name.toLowerCase(),
    cpu: (p) => p.cpu_pct,
    memory: (p) => p.private_bytes,
    disk: (p) => p.read_bps + p.write_bps,
  };

  const stream = new NowStream();
  const programs = $derived(stream.view.frame?.programs ?? null);
  let sort = $state<{ key: Key; down: boolean }>({ key: "cpu", down: true });
  let query = $state("");

  const matching = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    const list = (programs?.entries ?? []).filter((p) => !needle || p.name.toLowerCase().includes(needle));
    const value = VALUE[sort.key];
    return list.sort((a, b) => {
      const [x, y] = [value(a), value(b)];
      const order = x < y ? -1 : x > y ? 1 : a.pid - b.pid;
      return sort.down ? -order : order;
    });
  });
  const rows = $derived(matching.slice(0, SHOWN));

  /** A column sorts on the first click, from the most; the next click turns it around. */
  function by(key: Key): void {
    sort = sort.key === key ? { key, down: !sort.down } : { key, down: key !== "name" };
  }

  function ariaSort(key: Key): "ascending" | "descending" | undefined {
    return sort.key === key ? (sort.down ? "descending" : "ascending") : undefined;
  }
</script>

{#snippet head(key: Key, label: string)}
  <th scope="col" class:num={key !== "name"} aria-sort={ariaSort(key)}>
    <button type="button" onclick={() => by(key)}>{label}</button>
  </th>
{/snippet}

<header class="head">
  <h1>{t("ui.item.programs")}</h1>
</header>

{#if !stream.watching}
  <p class="lede">
    {t(app.paused ? "ui.now.lede.paused" : app.stopped ? "ui.now.lede.stopped" : "ui.now.lede.unreachable")}
  </p>
{:else if stream.error !== null}
  <ErrorView error={stream.error} onretry={() => stream.retry()} />
{:else if !stream.live.started}
  <Loading />
{:else}
  <div class="tools">
    <div class="search">
      <TextField bind:value={query} label={t("ui.now.all.search")} placeholder={t("ui.now.all.search")} />
    </div>
    <span class="state" class:paused={stream.paused}>
      <i></i>{t(stream.paused ? "ui.now.all.paused" : "ui.now.all.live")}
    </span>
  </div>
  {#if rows.length === 0}
    <p class="muted">{t(programs ? "ui.now.all.none" : "ui.now.top.waiting")}</p>
  {:else}
    <table>
      <thead>
        <tr>
          {@render head("name", t("ui.now.top.program"))}
          {@render head("cpu", t("ui.now.top.cpu"))}
          {@render head("memory", t("ui.now.top.memory"))}
          {@render head("disk", t("ui.now.top.disk"))}
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.pid)}
          <tr>
            <th scope="row" title={row.name}><a href={href.detail("process", row.name, { from: here(), label: t("ui.item.programs") })}>{row.name}</a></th>
            <td>{formatPercent(row.cpu_pct, 1)}</td>
            <td>{formatBytes(row.private_bytes)}</td>
            <td>{formatRate(row.read_bps + row.write_bps)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  <p class="count">{t("ui.now.all.count", { shown: rows.length, total: matching.length })}</p>
{/if}

<style>
  .head {
    margin-bottom: 4px;
  }
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 20px;
    margin: 16px 0 12px;
  }
  .search {
    width: 280px;
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .state i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--good);
  }
  .state.paused {
    color: var(--ink);
    font-weight: 600;
  }
  .state.paused i {
    border-radius: 2px;
    background: var(--ink2);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
    font-variant-numeric: tabular-nums;
  }
  thead th {
    padding: 0;
    border-bottom: 1px solid var(--rule);
    color: var(--ink2);
    font-weight: 600;
    text-align: left;
  }
  thead th.num {
    text-align: right;
  }
  thead button {
    width: 100%;
    padding: 8px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: inherit;
    cursor: pointer;
  }
  thead button:hover,
  thead th[aria-sort] button {
    color: var(--ink);
  }
  thead button:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  thead th[aria-sort="descending"] button::after {
    content: " ▾";
  }
  thead th[aria-sort="ascending"] button::after {
    content: " ▴";
  }
  tbody th,
  td {
    padding: 8px;
    border-bottom: 1px solid var(--stroke2);
    white-space: nowrap;
  }
  tbody th {
    max-width: 0;
    width: 46%;
    overflow: hidden;
    font-weight: 400;
    text-align: left;
    text-overflow: ellipsis;
  }
  td {
    text-align: right;
  }
  .count,
  .muted {
    margin: 12px 0 0;
    color: var(--ink2);
    font-size: 12.5px;
  }
  @media (forced-colors: active) {
    .state i {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
