<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { marks } from "../../lib/api";
  import { dumps } from "../../lib/dumps.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { loadDone, saveDone } from "../occurrence/browser";
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

  let { entry, level, open, ontoggle }: { entry: Entry; level: DetailLevel; open: boolean; ontoggle: () => void } =
    $props();

  const FALL_LOOK_MS = 10 * 60_000;

  const d = $derived(entry.dump);
  const gone = $derived(d !== null && d.deleted_ms !== null);
  const tone = $derived(TONE[entry.group]);
  const culprit = $derived(d && isRead(entry) && d.culprit ? culpritView(d, d.culprit, level) : null);
  const todo = $derived(d && isRead(entry) ? steps(d) : []);
  const id = $derived(`dump-${entry.key}`);

  let done = $state<string[]>([]);
  /** The post-mortem report of the fall this dump belongs to, once looked up. */
  let fall = $state<number | null>(null);

  $effect(() => {
    if (!open || !d) return;
    done = loadDone(d.id, "dump");
    if (d.kind !== "KernelMemory" && d.kind !== "KernelMini") return;
    const around = d.crash_ms;
    void marks(around - FALL_LOOK_MS, around + FALL_LOOK_MS)
      .then((m) => {
        fall = m.occurrences.find((o) => o.kind === "PostMortem")?.id ?? null;
      })
      .catch(() => (fall = null));
  });

  function tick(key: string, on: boolean): void {
    if (!d) return;
    done = on ? [...done, key] : done.filter((k) => k !== key);
    saveDone(d.id, done, "dump");
  }
</script>

<div class="dblock">
  <div class="drow" class:gone>
    {#if d && !gone}
      <input
        type="checkbox"
        class="pick"
        checked={dumps.chosen.includes(d.id)}
        aria-label={t("ui.dumps.pick", { file: fileName(d.path) })}
        onchange={(event) => dumps.choose(d.id, event.currentTarget.checked)}
      />
    {:else}
      <span></span>
    {/if}
    <span class="icochip" style:background="var(--{tone}-t)" style:color="var(--{tone})">
      <Icon name={ICON[entry.group]} />
    </span>
    <button type="button" class="open" aria-expanded={open} aria-controls={id} onclick={ontoggle}>
      <span class="what"><b>{title(entry, level)}</b><small>{sub(entry, level)}</small></span>
      <span class="side">
        {at(entry.at)}
        {#if d}<br />{formatBytes(d.size_bytes)}{/if}
      </span>
      <span class="chev"><Icon name="chevron" /></span>
    </button>
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
            <span class="icochip" style:background="var(--{tone}-t)" style:color="var(--{tone})">
              <Icon name={culprit.icon} />
            </span>
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
            <div class="todo">
              {#each todo as step (step.key)}
                <label class="chk">
                  <input
                    type="checkbox"
                    checked={done.includes(step.key)}
                    onchange={(event) => tick(step.key, event.currentTarget.checked)}
                  />
                  <span>{step.text}</span>
                </label>
              {/each}
            </div>
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
  :global(.dblock) + .dblock {
    border-top: 1px solid var(--stroke2);
  }
  .drow {
    display: grid;
    grid-template-columns: 22px 40px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
    padding: 10px;
    border-radius: 8px;
  }
  .drow:hover {
    background: var(--card2);
  }
  .drow.gone .what,
  .drow.gone .icochip {
    opacity: 0.62;
  }
  .open {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 12px;
    gap: 12px;
    align-items: center;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .what b {
    display: block;
    font-weight: 600;
    line-height: 1.3;
  }
  .what small {
    display: block;
    margin-top: 2px;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.35;
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
    transition: transform 0.15s;
  }
  .open[aria-expanded="true"] .chev {
    transform: rotate(90deg);
  }
  .pick {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: 0;
    border: 1.5px solid var(--ink3);
    border-radius: 6px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .pick:checked,
  .chk input:checked {
    border-color: var(--accent);
    background: var(--accent);
  }
  .pick:checked::after,
  .chk input:checked::after {
    content: "";
    width: 9px;
    height: 5px;
    border-bottom: 2px solid var(--on-accent);
    border-left: 2px solid var(--on-accent);
    transform: rotate(-45deg) translate(1px, -1px);
  }
  .detail {
    display: grid;
    gap: 14px;
    padding: 4px 10px 16px 84px;
  }
  .why {
    max-width: 70ch;
    margin: 0;
    line-height: 1.5;
  }
  .culprit {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 12px;
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
    margin: 0 0 8px;
    color: var(--ink2);
    font: 600 13px/1.35 var(--f);
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
  .todo {
    display: grid;
    gap: 10px;
  }
  .chk {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    line-height: 1.45;
    cursor: pointer;
  }
  .chk input {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: 1px 0 0;
    border: 1.5px solid var(--ink3);
    border-radius: 6px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .chk input:checked + span {
    color: var(--ink3);
    text-decoration: line-through;
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
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--card2);
    color: var(--ink2);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .file b {
    color: var(--ink);
    font-weight: 600;
  }
  @container content (max-width: 560px) {
    .open {
      grid-template-columns: minmax(0, 1fr) 12px;
    }
    .side {
      grid-row: 2;
      text-align: left;
    }
    .detail {
      padding-left: 10px;
    }
  }
  @media (forced-colors: active) {
    .bullet {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .pick:checked,
    .chk input:checked {
      forced-color-adjust: none;
      background: Highlight;
      border-color: Highlight;
    }
  }
</style>
