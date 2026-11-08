<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { formatDuration, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel, Episode } from "../../lib/types";
  import { episodeIcon, episodeMeta, episodeTitle } from "./text";

  let {
    e,
    level,
    byTest = false,
    fromDay = null,
  }: {
    e: Episode;
    level: DetailLevel;
    /** A Vestigium test most likely caused it. */
    byTest?: boolean;
    /** The History day this row is listed under, so the page can lead back to it. */
    fromDay?: string | null;
  } = $props();
</script>

<a class="row" href={href.period(e.start_ms, fromDay)} data-reveal>
  <span class="time">{formatTime(e.start_ms)}</span>
  <span class="ico"><Icon name={episodeIcon(e)} /></span>
  <span class="what">
    <b>{episodeTitle(e, level)}</b>
    <small>{episodeMeta(e)} · {formatDuration(e.end_ms - e.start_ms)}</small>
    {#if byTest}<small>{t("ui.today.episode.by_test")}</small>{/if}
  </span>
  <span class="end">
    <StatusMark kind="overload" reveal />
    <span class="chev"><Icon name="chevron" /></span>
  </span>
</a>

<style>
  .row {
    display: grid;
    grid-template-columns: 44px 20px minmax(0, 1fr) auto;
    gap: 14px;
    align-items: center;
    width: 100%;
    padding: 9px 12px;
    border-radius: 6px;
    background: var(--row);
    color: var(--ink);
    text-decoration: none;
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
</style>
