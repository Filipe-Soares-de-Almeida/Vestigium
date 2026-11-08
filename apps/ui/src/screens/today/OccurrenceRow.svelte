<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import StatusMark, { type StatusKind } from "../../components/StatusMark.svelte";
  import { formatDuration, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel, OccurrenceSummary } from "../../lib/types";
  import { toneOf } from "./day";
  import { causeIcon, causeName, occurrenceTitle } from "./text";

  let {
    o,
    level,
    fromDay = null,
  }: {
    o: OccurrenceSummary;
    level: DetailLevel;
    /** The History day this row is listed under, so the report can lead back to it. */
    fromDay?: string | null;
  } = $props();

  const STATUS: Record<ReturnType<typeof toneOf>, StatusKind> = {
    freeze: "critical",
    press: "attention",
    busy: "info",
  };
</script>

<a class="row" href={href.occurrence(o.id, fromDay)} data-reveal>
  <span class="time">{formatTime(o.kind === "PostMortem" ? o.end_ms : o.start_ms)}</span>
  <span class="ico"><Icon name={causeIcon(o.cause)} /></span>
  <span class="what">
    <b>{occurrenceTitle(o, level)}</b>
    <small>
      {t(`ui.common.kind.${o.kind}`)} · {causeName(o.cause)}{o.kind === "PostMortem"
        ? ""
        : ` · ${formatDuration(o.end_ms - o.start_ms)}`}
    </small>
  </span>
  <span class="end">
    <StatusMark kind={STATUS[toneOf(o)]} reveal />
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
