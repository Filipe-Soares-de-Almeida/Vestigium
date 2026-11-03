<script lang="ts">
  import { tick as settle } from "svelte";

  import ConcernMark from "../components/ConcernMark.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import Segmented from "../components/Segmented.svelte";
  import { app } from "../lib/app.svelte";
  import { dumps } from "../lib/dumps.svelte";
  import { errorText } from "../lib/errors";
  import { t, tn } from "../lib/i18n.svelte";
  import { toast } from "../lib/toast.svelte";
  import type { DetailLevel } from "../lib/types";
  import Row from "./dumps/Row.svelte";
  import { at, entries, type Entry, fileName, type Group, GROUPS, lastRead, lede, present, setting, space, TONE } from "./dumps/words";
  import { formatBytes } from "./occurrence/format";

  let { id }: { id: number | null } = $props();

  const LEVELS: DetailLevel[] = ["Simple", "Detailed", "Technical"];

  /** The level picked here, until the saved setting catches up with it. */
  let picked = $state<DetailLevel | null>(null);
  /** The entry shown open; the dump the address names opens first. */
  let opened = $state<string | null>(null);
  let dialog = $state<HTMLDialogElement>();

  const level = $derived(picked ?? app.settings?.settings.detail_level ?? "Simple");
  const list = $derived(dumps.dto ? entries(dumps.dto) : []);
  const shown = $derived(list.filter(present));
  const gone = $derived(list.filter((e) => !present(e)));
  const parts = $derived(dumps.dto ? space(list) : []);
  const chosen = $derived(list.flatMap((e) => (e.dump && dumps.chosen.includes(e.dump.id) ? [e.dump] : [])));
  const chosenBytes = $derived(chosen.reduce((sum, d) => sum + d.size_bytes, 0));
  const last = $derived(dumps.dto ? lastRead(dumps.dto) : null);
  const crash = $derived(setting(dumps.dto?.crash ?? null, level));
  // The helper's result reaches the history through the watcher.
  const blocked = $derived(!app.running);

  $effect(() => {
    const wanted = id;
    if (wanted === null || !dumps.dto || opened !== null) return;
    const entry = list.find((e) => e.dump?.id === wanted);
    if (!entry) return;
    opened = entry.key;
    void settle().then(() => document.getElementById(`dump-${entry.key}`)?.scrollIntoView({ block: "center" }));
  });

  /** Shows the level at once and saves it as the default while the watcher can write settings. */
  async function pickLevel(next: DetailLevel): Promise<void> {
    picked = next;
    const saved = app.settings?.settings;
    if (!app.editable || !saved || saved.detail_level === next) return;
    try {
      await app.saveSettings({ ...saved, detail_level: next });
    } catch (error) {
      toast.show(errorText(error));
    }
  }

  function toggle(entry: Entry): void {
    opened = opened === entry.key ? "" : entry.key;
  }

  function retry(): void {
    dumps.error = null;
    void dumps.load();
  }

  async function remove(): Promise<void> {
    dialog?.close();
    const freed = await dumps.remove();
    if (freed === null) {
      toast.show(t(dumps.deleting === "declined" ? "ui.dumps.delete.declined" : "ui.dumps.delete.failed"));
    } else if (freed.count < freed.asked) {
      toast.show(t("ui.dumps.delete.toast.partial", { count: freed.count, asked: freed.asked, size: formatBytes(freed.bytes) }));
    } else {
      toast.show(t("ui.dumps.delete.toast", { size: formatBytes(freed.bytes) }));
    }
  }

  const groupLabel = (g: Group) => t(`ui.dumps.group.${g}`);
</script>

{#snippet rows(group: Entry[])}
  {#each group as entry (entry.key)}
    <Row {entry} {level} open={opened === entry.key} ontoggle={() => toggle(entry)} />
  {/each}
{/snippet}

<div class="top">
  <h1>{t("ui.dumps.title")}</h1>
  <Segmented
    label={t("ui.common.detail.label")}
    options={LEVELS.map((value) => ({
      value,
      label: t(`ui.common.detail.${value}`),
      hint: t(`ui.common.detail.${value}.hint`),
    }))}
    value={level}
    onchange={pickLevel}
  />
</div>

{#if dumps.dto === null && dumps.error !== null}
  <div class="gap"><ErrorView error={dumps.error} heading="h2" onretry={retry} /></div>
{:else if dumps.dto === null}
  <Loading />
{:else}
  <p class="lede">
    {#each lede(list, dumps.dto, level) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
  </p>
  <div class="split">
    <div class="stack">
      {#each GROUPS as group (group)}
        {@const members = shown.filter((e) => e.group === group)}
        {#if members.length}
          <section class="card list" aria-labelledby="group-{group}">
            <h2 id="group-{group}">{groupLabel(group)}</h2>
            {@render rows(members)}
          </section>
        {/if}
      {/each}
      {#if !shown.length}
        <section class="card">
          <p class="muted empty">{t("ui.dumps.empty")}</p>
        </section>
      {/if}
      {#if gone.length}
        <section class="card list">
          <details class="gone">
            <summary>{tn("ui.dumps.gone", gone.length)}</summary>
            {@render rows(gone)}
          </details>
        </section>
      {/if}
    </div>

    <div class="stack">
      {#if parts.length}
        <section class="card space" aria-labelledby="space-title">
          <h2 id="space-title" class="sr">{t("ui.dumps.space.title")}</h2>
          <p class="big">
            {formatBytes(dumps.dto.space_bytes)}
            <small>{tn("ui.dumps.space.in", shown.filter((e) => e.dump).length)}</small>
          </p>
          <div
            class="bar"
            role="img"
            aria-label={parts.map((p) => `${groupLabel(p.group)}: ${formatBytes(p.bytes)}`).join("; ")}
          >
            {#each parts as p (p.group)}
              <i style:flex-grow={p.bytes} style:background="var(--{TONE[p.group]})"></i>
            {/each}
          </div>
          <div class="key" aria-hidden="true">
            {#each parts as p (p.group)}
              <span><i style:background="var(--{TONE[p.group]})"></i>{groupLabel(p.group)} {formatBytes(p.bytes)}</span>
            {/each}
          </div>
        </section>
      {/if}

      <section class="card read" aria-labelledby="read-title">
        <h2 id="read-title">{t("ui.dumps.read.title")}</h2>
        {#if dumps.reading === "running"}
          <p class="status" role="status"><span class="spin" aria-hidden="true"></span>{t("ui.dumps.read.running")}</p>
        {:else}
          {#if dumps.reading === "declined"}
            <p role="status"><b>{t("ui.dumps.read.declined.title")}</b> {t("ui.dumps.read.declined.body")}</p>
          {:else if dumps.reading === "failed"}
            <p role="status"><b>{t("ui.dumps.read.failed.title")}</b> {t("ui.dumps.read.failed.body")}</p>
          {:else if dumps.reading === "done"}
            <p role="status"><b>{t("ui.dumps.read.done")}</b></p>
          {/if}
          <p>{t(level === "Technical" ? "ui.dumps.read.lede.tech" : "ui.dumps.read.lede")}</p>
          <div class="line">
            <button
              type="button"
              class="btn"
              class:accent={last === null}
              disabled={blocked}
              onclick={() => void dumps.read()}
            >
              <Icon name="shield" />
              {t(
                dumps.reading === "declined" || dumps.reading === "failed"
                  ? "ui.dumps.read.again"
                  : last === null
                    ? "ui.dumps.read.start"
                    : "ui.dumps.read.repeat",
              )}
            </button>
            <span>
              {blocked
                ? t("ui.dumps.read.blocked")
                : last === null
                  ? t("ui.dumps.read.never")
                  : t("ui.dumps.read.last", { when: at(last) })}
            </span>
          </div>
        {/if}
      </section>

      <section class="card" aria-labelledby="setting-title">
        <h2 id="setting-title" class="sr">{t("ui.dumps.setting.title")}</h2>
        <p class="setting">
          <ConcernMark mark={crash.ok ? "Ok" : "Attention"} word={!crash.ok} />
          <span>{crash.text}</span>
        </p>
      </section>
    </div>
  </div>

  {#if chosen.length}
    <div class="delbar" role="region" aria-label={t("ui.dumps.delete.bar")}>
      <p>
        {tn("ui.dumps.delete.chosen", chosen.length)}
        <small>{t("ui.dumps.delete.kept", { size: formatBytes(chosenBytes) })}</small>
      </p>
      <div class="acts">
        <button type="button" class="btn" onclick={() => (dumps.chosen = [])}>{t("ui.dumps.delete.clear")}</button>
        <button
          type="button"
          class="btn danger"
          disabled={blocked || dumps.deleting === "running"}
          onclick={() => dialog?.showModal()}
        >
          <Icon name="trash" />
          {t(dumps.deleting === "running" ? "ui.dumps.delete.running" : "ui.dumps.delete.start")}
        </button>
      </div>
    </div>
  {/if}

  <dialog class="confirm" bind:this={dialog} aria-labelledby="confirm-title">
    <h2 id="confirm-title">{tn("ui.dumps.delete.ask", chosen.length)}</h2>
    <p>{t("ui.dumps.delete.ask.body", { size: formatBytes(chosenBytes) })}</p>
    <ul>
      {#each chosen as d (d.id)}
        <li>{fileName(d.path)} · {formatBytes(d.size_bytes)}</li>
      {/each}
    </ul>
    <div class="acts">
      <button type="button" class="btn" onclick={() => dialog?.close()}>{t("ui.dumps.delete.cancel")}</button>
      <button type="button" class="btn danger" onclick={() => void remove()}>
        <Icon name="trash" />{t("ui.dumps.delete.confirm")}
      </button>
    </div>
  </dialog>
{/if}

<style>
  .top {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }
  .gap {
    margin-top: 14px;
  }
  .list {
    padding: 6px;
  }
  .list h2 {
    margin: 0;
    padding: 8px 10px 4px;
    font: 600 15px/1.3 var(--f);
  }
  .empty {
    margin: 0;
  }
  .gone summary {
    padding: 8px 10px;
    border-radius: 8px;
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
  }
  .space {
    display: grid;
    gap: 8px;
  }
  .big {
    margin: 0;
    font: 600 34px/1 var(--fd);
    letter-spacing: -0.02em;
  }
  .big small {
    margin-left: 6px;
    color: var(--ink2);
    font: 400 14px var(--f);
    letter-spacing: 0;
  }
  .bar {
    display: flex;
    gap: 2px;
    height: 12px;
    overflow: hidden;
    border: 1px solid var(--stroke2);
    border-radius: 999px;
    background: var(--card2);
  }
  .bar i {
    flex-basis: 0;
    min-width: 4px;
  }
  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--ink2);
    font-size: 13px;
  }
  .key i {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 6px;
    border-radius: 2px;
  }
  .read {
    display: grid;
    gap: 10px;
  }
  .read h2 {
    margin: 0;
    font: 600 15px/1.3 var(--f);
  }
  .read p {
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.45;
  }
  .read p b {
    color: var(--ink);
    font-weight: 600;
  }
  .status {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .read .status {
    color: var(--ink);
  }
  .spin {
    flex: none;
    width: 14px;
    height: 14px;
    border: 2px solid var(--stroke);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
  }
  .line :global(svg) {
    width: 16px;
    height: 16px;
  }
  .setting {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .setting :global(.cn) {
    padding-top: 1px;
  }
  .delbar {
    position: sticky;
    bottom: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    margin-top: 14px;
    padding: 12px 16px;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--card);
    box-shadow: 0 8px 28px #0003;
  }
  .delbar p {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .delbar small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
    font-weight: 400;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
  }
  .btn.danger {
    border-color: transparent;
    background: var(--freeze);
    color: var(--on-accent);
  }
  .btn.danger:hover {
    filter: brightness(1.07);
  }
  .confirm {
    max-width: min(520px, calc(100vw - 32px));
    padding: 18px 20px;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 18px 60px #0006;
  }
  .confirm::backdrop {
    background: #0008;
  }
  .confirm h2 {
    margin: 0 0 8px;
    font: 600 18px/1.3 var(--fd);
  }
  .confirm p {
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 14px;
    line-height: 1.5;
  }
  .confirm ul {
    display: grid;
    gap: 4px;
    max-height: 160px;
    margin: 0 0 12px;
    padding-left: 18px;
    overflow: auto;
    font-size: 13px;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation-duration: 3s;
    }
  }
  @media (forced-colors: active) {
    .bar i,
    .key i,
    .spin {
      forced-color-adjust: none;
    }
    .bar i,
    .key i {
      background: CanvasText;
    }
    .spin {
      border-color: GrayText;
      border-top-color: Highlight;
    }
  }
</style>
