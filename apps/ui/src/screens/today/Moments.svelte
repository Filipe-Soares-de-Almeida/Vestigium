<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";

  import Block from "../../components/Block.svelte";
  import Icon from "../../components/Icon.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { formatDuration, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel } from "../../lib/types";
  import type { Item } from "./day";
  import OccurrenceRow from "./OccurrenceRow.svelte";
  import Period from "./Period.svelte";
  import { episodeIcon, episodeMeta, episodeTitle } from "./text";

  let { items, level }: { items: Item[]; level: DetailLevel } = $props();

  /** Episodes open in place, by row key, so the day's refresh keeps them open. */
  const opened = new SvelteSet<string>();
  const keyOf = (item: Item) => `${item.kind}-${item.start}`;

  function toggle(key: string): void {
    if (opened.has(key)) opened.delete(key);
    else opened.add(key);
  }
</script>

<Block title={t("ui.today.moments.title")}>
  {#if items.length === 0}
    <p class="empty">{t("ui.today.moments.none")}</p>
  {:else}
    <ul>
      {#each items as item (keyOf(item))}
        <li>
          {#if item.kind === "occurrence"}
            {@const o = item.occurrence}
            <OccurrenceRow {o} {level} />
          {:else}
            {@const e = item.episode}
            {@const key = keyOf(item)}
            {@const open = opened.has(key)}
            <div class="period" class:open>
              <button
                type="button"
                class="row"
                data-reveal
                aria-expanded={open}
                aria-controls={open ? `period-${key}` : undefined}
                onclick={() => toggle(key)}
              >
                <span class="time">{formatTime(e.start_ms)}</span>
                <span class="ico"><Icon name={episodeIcon(e)} /></span>
                <span class="what">
                  <b>{episodeTitle(e, level)}</b>
                  <small>{episodeMeta(e)} · {formatDuration(e.end_ms - e.start_ms)}</small>
                  {#if item.byTest}<small>{t("ui.today.episode.by_test")}</small>{/if}
                </span>
                <span class="end">
                  <StatusMark kind="overload" reveal />
                  <span class="chev turn"><Icon name="chevron" /></span>
                </span>
              </button>
              {#if open}
                <Period
                  episode={e}
                  byTest={item.byTest}
                  {level}
                  id="period-{key}"
                  label={episodeTitle(e, level)}
                />
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Block>

<style>
  ul {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    grid-template-columns: 44px 20px minmax(0, 1fr) auto;
    gap: 14px;
    align-items: center;
    width: 100%;
    padding: 9px 12px;
    border: 0;
    border-radius: 6px;
    background: var(--row);
    color: var(--ink);
    font: inherit;
    text-align: left;
    text-decoration: none;
  }
  button.row {
    cursor: pointer;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .row:active {
    background: var(--row-press);
  }
  .row:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  /* An open episode is one block: its row is the header and the detail sits under it. */
  .period.open {
    margin: 2px 0 8px;
    overflow: hidden;
    border-radius: 6px;
    background: var(--row);
  }
  .period.open .row {
    border-radius: 0;
  }
  .time {
    color: var(--ink2);
    font-size: 13px;
  }
  .ico {
    display: flex;
    color: var(--ink2);
  }
  .what {
    display: grid;
    min-width: 0;
  }
  .what b {
    overflow: hidden;
    font-size: 14.5px;
    font-weight: 400;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  small {
    overflow: hidden;
    color: var(--ink2);
    font-size: 12.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .end {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .chev {
    display: grid;
    color: var(--ink3);
  }
  .turn :global(svg) {
    transform: rotate(90deg);
    transition: transform 0.2s;
  }
  .open .turn :global(svg) {
    transform: rotate(-90deg);
  }
  .empty {
    margin: 0;
    color: var(--ink2);
  }
  @media (prefers-reduced-motion: reduce) {
    .turn :global(svg) {
      transition: none;
    }
  }
</style>
