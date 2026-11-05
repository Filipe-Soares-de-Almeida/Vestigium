<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";

  import Icon from "../../components/Icon.svelte";
  import { formatDuration, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { type Item, toneOf } from "./day";
  import Period from "./Period.svelte";
  import {
    causeIcon,
    causeName,
    episodeIcon,
    episodeMeta,
    episodeTitle,
    occurrenceTitle,
  } from "./text";

  let { items, level }: { items: Item[]; level: DetailLevel } = $props();

  /** Episodes open in place, by row key, so the day's refresh keeps them open. */
  const opened = new SvelteSet<string>();
  const keyOf = (item: Item) => `${item.kind}-${item.start}`;

  function toggle(key: string): void {
    if (opened.has(key)) opened.delete(key);
    else opened.add(key);
  }
</script>

<section class="card list" aria-labelledby="moments-title">
  <h3 id="moments-title">{t("ui.today.moments.title")}</h3>
  {#if items.length === 0}
    <p class="empty">{t("ui.today.moments.none")}</p>
  {:else}
    <ul>
      {#each items as item (keyOf(item))}
        <li>
          {#if item.kind === "occurrence"}
            {@const o = item.occurrence}
            <a class="row" href={href.occurrence(o.id)}>
              <span class="icochip tone-{toneOf(o)}"><Icon name={causeIcon(o.cause)} /></span>
              <span class="what">
                <b>{occurrenceTitle(o, level)}</b>
                <small>{t(`ui.common.kind.${o.kind}`)} · {causeName(o.cause)}</small>
              </span>
              {#if o.kind === "PostMortem"}
                <span class="when">{formatTime(o.end_ms)}</span>
              {:else}
                <span class="when">{formatTime(o.start_ms)}<br />{formatDuration(o.end_ms - o.start_ms)}</span>
              {/if}
              <span class="chev"><Icon name="chevron" /></span>
            </a>
          {:else}
            {@const e = item.episode}
            {@const key = keyOf(item)}
            {@const open = opened.has(key)}
            <div class="period" class:open>
              <button
                type="button"
                class="row"
                aria-expanded={open}
                aria-controls={open ? `period-${key}` : undefined}
                onclick={() => toggle(key)}
              >
                <span class="icochip tone-press"><Icon name={episodeIcon(e)} /></span>
                <span class="what">
                  <b>{episodeTitle(e, level)}</b>
                  <small>{episodeMeta(e)}</small>
                  {#if item.byTest}<small>{t("ui.today.episode.by_test")}</small>{/if}
                </span>
                <span class="when">{formatTime(e.start_ms)}<br />{formatDuration(e.end_ms - e.start_ms)}</span>
                <span class="chev turn"><Icon name="chevron" /></span>
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
</section>

<style>
  .list {
    padding: 10px;
  }
  h3 {
    padding: 6px 10px 2px;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto 14px;
    gap: 12px;
    align-items: center;
    padding: 9px 10px;
    border-radius: 8px;
    color: inherit;
    text-decoration: none;
  }
  button.row {
    width: 100%;
    border: 0;
    background: none;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  a.row:hover,
  button.row:hover {
    background: var(--card2);
  }
  /* An open episode is one card: its row is the header and the detail sits under it. */
  .period.open {
    margin: 2px 0 8px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 10px;
    background: var(--card2);
  }
  .period.open button.row {
    border-radius: 0;
  }
  .period.open button.row:hover {
    background: color-mix(in srgb, var(--ink) 4%, transparent);
  }
  .turn :global(svg) {
    transform: rotate(90deg);
    transition: transform 0.2s;
  }
  .open .turn :global(svg) {
    transform: rotate(-90deg);
  }
  .what b {
    display: block;
    font-weight: 600;
    line-height: 1.3;
  }
  small {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .when {
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.35;
    text-align: right;
    white-space: nowrap;
  }
  .chev {
    display: grid;
    color: var(--ink3);
  }
  .tone-busy {
    background: var(--busy-t);
    color: var(--busy);
  }
  .tone-press {
    background: var(--press-t);
    color: var(--press);
  }
  .tone-freeze {
    background: var(--freeze-t);
    color: var(--freeze);
  }
  .empty {
    margin: 0;
    padding: 4px 10px 8px;
    color: var(--ink2);
  }
  @media (forced-colors: active) {
    a.row:hover,
    button.row:hover {
      outline: 1px solid Highlight;
    }
  }
</style>
