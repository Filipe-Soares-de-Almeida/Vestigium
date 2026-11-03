<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { formatDuration, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { type Item, toneOf } from "./day";
  import {
    causeIcon,
    causeName,
    episodeIcon,
    episodeMeta,
    episodeTitle,
    occurrenceTitle,
  } from "./text";

  let { items, level }: { items: Item[]; level: DetailLevel } = $props();
</script>

<section class="card list" aria-labelledby="moments-title">
  <h3 id="moments-title">{t("ui.today.moments.title")}</h3>
  {#if items.length === 0}
    <p class="empty">{t("ui.today.moments.none")}</p>
  {:else}
    <ul>
      {#each items as item (`${item.kind}-${item.start}`)}
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
            <div class="row">
              <span class="icochip tone-press"><Icon name={episodeIcon(e)} /></span>
              <span class="what">
                <b>{episodeTitle(e, level)}</b>
                <small>{episodeMeta(e)}</small>
                {#if item.byTest}<small>{t("ui.today.episode.by_test")}</small>{/if}
              </span>
              <span class="when">{formatTime(e.start_ms)}<br />{formatDuration(e.end_ms - e.start_ms)}</span>
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
  a.row:hover {
    background: var(--card2);
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
    a.row:hover {
      outline: 1px solid Highlight;
    }
  }
</style>
