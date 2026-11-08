<script lang="ts">
  import Checkbox from "../../components/Checkbox.svelte";
  import Icon from "../../components/Icon.svelte";
  import Steps from "../../components/Steps.svelte";
  import { marks } from "../../lib/api";
  import { dumps } from "../../lib/dumps.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { formatBytes } from "../occurrence/format";
  import {
    at,
    culpritView,
    type Entry,
    fileName,
    hex,
    ICON,
    isRead,
    KIND_LABEL,
    said,
    steps,
    stopSentence,
    sub,
    title,
    TONE,
  } from "./words";

  let {
    entry,
    level,
    open = false,
    href: to,
  }: {
    entry: Entry;
    level: DetailLevel;
    /** Shows the whole report under the row, as the dump's own page does. */
    open?: boolean;
    /** A list row leads to the dump's page; without it the row is only the header of that page. */
    href?: string;
  } = $props();

  const FALL_LOOK_MS = 10 * 60_000;

  const d = $derived(entry.dump);
  const gone = $derived(d !== null && d.deleted_ms !== null);
  const tone = $derived(TONE[entry.group]);
  const culprit = $derived(d && isRead(entry) && d.culprit ? culpritView(d, d.culprit, level) : null);
  const todo = $derived(d && isRead(entry) ? steps(d) : []);
  const id = $derived(`dump-${entry.key}`);

  /** The post-mortem report of the fall this dump belongs to, once looked up. */
  let fall = $state<number | null>(null);

  $effect(() => {
    if (!open || !d) return;
    if (d.kind !== "KernelMemory" && d.kind !== "KernelMini") return;
    const around = d.crash_ms;
    void marks(around - FALL_LOOK_MS, around + FALL_LOOK_MS)
      .then((m) => {
        fall = m.occurrences.find((o) => o.kind === "PostMortem")?.id ?? null;
      })
      .catch(() => (fall = null));
  });
</script>

{#snippet face()}
  <span class="ico"><Icon name={ICON[entry.group]} /></span>
  <span class="what"><b>{title(entry, level)}</b><small>{sub(entry, level)}</small></span>
  <span class="side">
    {at(entry.at)}
    {#if d}<br />{formatBytes(d.size_bytes)}{/if}
  </span>
  {#if to}<span class="chev"><Icon name="chevron" /></span>{/if}
{/snippet}

<div class="dblock">
  <div class="drow" class:gone>
    {#if d && !gone}
      <Checkbox checked={dumps.chosen.includes(d.id)} onchange={(on) => dumps.choose(d.id, on)}>
        <span class="sr">{t("ui.dumps.pick", { file: fileName(d.path) })}</span>
      </Checkbox>
    {:else}
      <span></span>
    {/if}
    {#if to}
      <a class="open" href={to} data-reveal>{@render face()}</a>
    {:else}
      <div class="open static">{@render face()}</div>
    {/if}
  </div>
  {#if open}
    <div class="detail" {id}>
      {#if !d || !isRead(entry)}
        <p class="why">
          {#if !d && !entry.report?.dump_path}
            {t("ui.dumps.detail.no_dump")}
          {:else if d && d.analyzed_ms !== null}
            {t("ui.dumps.detail.unreadable")}
          {:else}
            {t(level === "Technical" ? "ui.dumps.detail.unread.tech" : "ui.dumps.detail.unread")}
          {/if}
        </p>
        {#if entry.report}
          <p class="why">
            {t("ui.dumps.detail.reported", { code: hex(entry.report.code), what: stopSentence(entry.report.code) })}
          </p>
        {/if}
      {:else}
        {#if culprit}
          <div class="culprit">
            <span class="ico"><Icon name={culprit.icon} /></span>
            <div><b>{culprit.name}</b><small>{culprit.sub}</small></div>
          </div>
        {:else}
          <p class="why">{t("ui.dumps.detail.no_culprit")}</p>
        {/if}
        <div class="two">
          <div>
            <h3>{t("ui.dumps.detail.said")}</h3>
            <ul class="ev">
              {#each said(d, level) as line, i (i)}
                <li>
                  <span class="bullet" class:first={i === 0} aria-hidden="true"></span>
                  <span>{line.main}{#if line.sub}<small class:mono={line.mono}>{line.sub}</small>{/if}</span>
                </li>
              {/each}
            </ul>
          </div>
          <div>
            <h3>{t("ui.occurrence.todo.title")}</h3>
            <Steps steps={todo} />
          </div>
        </div>
      {/if}
      {#if fall !== null}
        <p class="fall">
          <Icon name="power" />
          <span>
            {t("ui.dumps.detail.fall", { when: at(entry.at) })}
            <a href={href.occurrence(fall)}>{t("ui.dumps.detail.fall_link")}</a>
          </span>
        </p>
      {/if}
      {#if d}
        <div class="file">
          <span><b>{fileName(d.path)}</b> · {KIND_LABEL(d, level)} · {formatBytes(d.size_bytes)}</span>
          {#if level !== "Simple"}<span class="mono">{d.path}</span>{/if}
          <span>
            {gone
              ? t("ui.dumps.detail.deleted", { when: at(d.deleted_ms ?? 0) })
              : t("ui.dumps.detail.written", { when: at(d.file_ms) })}
          </span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .drow {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 14px;
    align-items: center;
    padding-left: 12px;
    border-radius: 6px;
    background: var(--row);
  }
  .drow:hover {
    background: var(--row-hover);
  }
  .drow.gone .what,
  .drow.gone .ico {
    opacity: 0.62;
  }
  .open {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto auto;
    gap: 14px;
    align-items: center;
    min-width: 0;
    padding: 9px 12px 9px 0;
    border-radius: 6px;
    color: inherit;
    text-decoration: none;
  }
  .open:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
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
  .what small {
    overflow: hidden;
    color: var(--ink2);
    font-size: 12.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .side {
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.35;
    text-align: right;
    white-space: nowrap;
  }
  .chev {
    display: grid;
    color: var(--ink3);
  }
  .detail {
    display: grid;
    gap: 14px;
    padding: 24px 0 0;
  }
  .why {
    max-width: 70ch;
    margin: 0;
    line-height: 1.5;
  }
  .culprit {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 14px;
    align-items: center;
  }
  .culprit b {
    display: block;
    font-weight: 600;
  }
  .culprit small {
    color: var(--ink2);
    font-size: 13px;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 16px;
  }
  h3 {
    margin: 0 0 12px;
    font: 400 20px/1.3 var(--fd);
  }
  .ev {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ev li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .ev small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
  .mono {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12.5px;
  }
  .bullet {
    width: 6px;
    height: 6px;
    margin: 0.5em 0 0 6px;
    border-radius: 50%;
    background: var(--ink3);
  }
  .bullet.first {
    background: var(--accent);
  }
  .fall {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
  }
  .fall :global(svg) {
    flex: none;
    width: 18px;
    height: 18px;
  }
  .fall a {
    color: var(--accent);
    font-weight: 600;
  }
  .file {
    display: grid;
    gap: 4px;
    color: var(--ink2);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .file b {
    color: var(--ink);
    font-weight: 600;
  }
  @container content (max-width: 560px) {
    .side {
      display: none;
    }
    .open {
      grid-template-columns: 20px minmax(0, 1fr) auto;
    }
  }
  @media (forced-colors: active) {
    .bullet {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
