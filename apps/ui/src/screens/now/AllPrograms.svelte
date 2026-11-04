<script lang="ts">
  import { formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { ProcessSample, ProcessTop } from "../../lib/types";
  import { formatBytes, formatRate } from "./lanes";

  let {
    programs,
    paused,
    onclose,
  }: {
    programs: ProcessTop | null;
    /** Ctrl is held: the list shows the second it was pressed. */
    paused: boolean;
    onclose: () => void;
  } = $props();

  /** Rows the table shows at most, after the search and the sort. */
  const SHOWN = 100;
  type Key = "name" | "cpu" | "memory" | "disk";
  const VALUE: Record<Key, (p: ProcessSample) => number | string> = {
    name: (p) => p.name.toLowerCase(),
    cpu: (p) => p.cpu_pct,
    memory: (p) => p.private_bytes,
    disk: (p) => p.read_bps + p.write_bps,
  };

  let dialog = $state<HTMLDialogElement>();
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

  $effect(() => {
    dialog?.showModal();
  });

  /** Closes the list: the × button, Esc and a click outside it. */
  function shut(): void {
    dialog?.close();
    onclose();
  }

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

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  class="all"
  bind:this={dialog}
  aria-labelledby="all-title"
  oncancel={(event) => {
    event.preventDefault();
    shut();
  }}
  onclick={(event) => {
    const box = dialog?.getBoundingClientRect();
    const outside =
      box && (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom);
    if (event.target === dialog && outside) shut();
  }}
>
  <div class="bar">
    <h2 id="all-title">{t("ui.now.all.title")}</h2>
    <span class="state" class:paused>
      <i></i>{t(paused ? "ui.now.all.paused" : "ui.now.all.live")}
    </span>
    <input
      class="search"
      type="search"
      placeholder={t("ui.now.all.search")}
      aria-label={t("ui.now.all.search")}
      bind:value={query}
    />
    <button type="button" class="close" aria-label={t("ui.now.all.close")} onclick={shut}>×</button>
  </div>
  <div class="rows">
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
              <th scope="row" title={row.name}>{row.name}</th>
              <td>{formatPercent(row.cpu_pct, 1)}</td>
              <td>{formatBytes(row.private_bytes)}</td>
              <td>{formatRate(row.read_bps + row.write_bps)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
  <p class="count">
    {t("ui.now.all.count", { shown: rows.length, total: matching.length })}
  </p>
</dialog>

<style>
  .all {
    width: min(860px, calc(100vw - 48px));
    height: min(600px, calc(100vh - 80px));
    padding: 16px 18px;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .all[open] {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .all::backdrop {
    background: #0007;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
  }
  h2 {
    margin: 0 auto 0 0;
    font: 600 18px/1.3 var(--fd);
  }
  .state {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .state i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
  }
  .state.paused {
    color: var(--ink);
    font-weight: 600;
  }
  .state.paused i {
    border-radius: 2px;
    background: var(--ink2);
  }
  .search {
    width: 220px;
    height: 32px;
    padding: 0 10px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--layer);
    color: var(--ink);
    font: 400 14px/1 var(--f);
  }
  .close {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--ink2);
    font-size: 20px;
    cursor: pointer;
  }
  .close:hover {
    background: var(--stroke2);
    color: var(--ink);
  }
  .rows {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
    font-variant-numeric: tabular-nums;
  }
  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0;
    background: var(--card);
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
  thead th[aria-sort="descending"] button::after {
    content: " ▾";
  }
  thead th[aria-sort="ascending"] button::after {
    content: " ▴";
  }
  tbody th,
  td {
    padding: 6px 8px;
    border-top: 1px solid var(--stroke2);
    white-space: nowrap;
  }
  tbody th {
    max-width: 0;
    width: 46%;
    overflow: hidden;
    font-weight: 600;
    text-align: left;
    text-overflow: ellipsis;
  }
  td {
    text-align: right;
  }
  .count,
  .muted {
    margin: 0;
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
