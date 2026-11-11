<script lang="ts">
  import { tick } from "svelte";

  import Icon from "../../components/Icon.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { Advice, DetailLevel, TweakRow } from "../../lib/types";
  import {
    changeOf,
    foundOf,
    interruptedLead,
    interruptedMark,
    interruptedOf,
    interruptedOutcome,
    name,
    settleMark,
    undoable,
  } from "./words";

  let {
    row,
    advice,
    level,
    disabled,
    onundo,
  }: {
    /** The change whose helper stopped, and which nothing has happened to since. */
    row: TweakRow;
    advice: Advice;
    level: DetailLevel;
    disabled: boolean;
    /** The person confirmed putting back what the helper changed. */
    onundo: () => void;
  } = $props();

  const kind = $derived(interruptedOf(row) ?? "unknown");
  const canUndo = $derived(undoable(row));
  const found = $derived(foundOf(advice, row, level));
  const listed = $derived(kind === "part" || kind === "doubt" || level === "Technical");
  /** Every setting holds the new value, so Undo puts back the whole change. */
  const whole = $derived(kind === "all");
  const back = $derived(
    (changeOf(advice, row.change.results.map((r) => r.change), "Simple")[0]?.from ?? "").toLocaleLowerCase(),
  );

  let confirming = $state(false);
  let confirm = $state<HTMLDivElement>();

  async function ask(): Promise<void> {
    confirming = true;
    await tick();
    confirm?.querySelector<HTMLButtonElement>("button.accent")?.focus();
  }

  function keys(e: KeyboardEvent): void {
    if (confirming && e.key === "Escape") {
      e.stopPropagation();
      confirming = false;
    }
  }
</script>

<svelte:window onkeydown={keys} />

<div class="note" role="status">
  <span class="mark"><StatusMark kind={interruptedMark(kind)} size={20} word={interruptedLead()} /></span>
  <div class="body">
    <p class="head"><b>{interruptedLead()}</b></p>
    <p>{interruptedOutcome(kind, canUndo)}</p>
    {#if listed}
      <ul class="found">
        {#each found as f, i (i)}
          <li>
            <StatusMark kind={settleMark(f.settle)} word="" />
            <span class:mono={level === "Technical"}>{f.name}</span>
            <span class="what">{f.word}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {#if confirming}
      <div class="ask" role="group" aria-labelledby="undo-stopped-title" bind:this={confirm}>
        <p class="head" id="undo-stopped-title">{t(whole ? "ui.tweaks.undo.title" : "ui.tweaks.undo.interrupted.title")}</p>
        <p>
          {whole
            ? t("ui.tweaks.undo.body", { name: name(advice, "Simple"), value: back })
            : t("ui.tweaks.undo.interrupted.body")}{row.applier === "Elevated" ? ` ${t("ui.tweaks.ask.prompt")}` : ""}
        </p>
        <div class="acts">
          <button type="button" class="btn accent" {disabled} onclick={onundo}>
            <Icon name="undo" />{t(whole ? "ui.tweaks.undo" : "ui.tweaks.undo.interrupted.go")}
          </button>
          <button type="button" class="btn" onclick={() => (confirming = false)}>{t("ui.tweaks.cancel")}</button>
        </div>
      </div>
    {:else if canUndo}
      <div class="acts">
        <button type="button" class="btn" {disabled} onclick={() => void ask()}><Icon name="undo" />{t("ui.tweaks.undo")}</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .note {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    margin: 4px 0 18px;
    padding: 14px 16px;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--row);
  }
  .mark {
    display: flex;
    margin-top: 2px;
  }
  .mark :global(.word) {
    display: none;
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  p {
    margin: 0 0 6px;
    font-size: 14px;
    line-height: 1.5;
  }
  .head,
  .head b {
    color: var(--ink);
    font-weight: 600;
  }
  .head b {
    font-size: 15px;
  }
  .found {
    display: grid;
    gap: 4px;
    margin: 8px 0 12px;
    padding: 0;
    list-style: none;
  }
  .found li {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    min-height: 34px;
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--row);
    font-size: 14px;
  }
  .found li :global(.word) {
    display: none;
  }
  .what {
    color: var(--ink2);
    font-size: 13px;
    text-align: right;
  }
  .mono {
    font: 12.5px var(--mono);
  }
  .acts {
    margin-top: 10px;
  }
  .ask {
    margin-top: 8px;
  }
</style>
