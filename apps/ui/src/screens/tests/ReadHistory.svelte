<script lang="ts">
  import ConcernMark from "../../components/ConcernMark.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel, DiskDto, TestRunDto } from "../../lib/types";
  import { at } from "../dumps/words";
  import DiskMap from "./DiskMap.svelte";
  import { shareOf, span, surfaceOf, surfaceWord } from "./words";

  // Every read of one disk, the newest first; choosing one shows its map.
  let {
    runs,
    disk,
    name,
    level,
    onclose,
  }: { runs: TestRunDto[]; disk: DiskDto; name: string; level: DetailLevel; onclose: () => void } = $props();

  let dialog = $state<HTMLDialogElement>();
  let picked = $state(0);

  const rows = $derived(
    runs.flatMap((run) => {
      const scan = surfaceOf(run);
      return scan ? [{ run, scan, word: surfaceWord(run, scan, disk) }] : [];
    }),
  );
  const current = $derived(rows[Math.min(picked, rows.length - 1)] ?? null);

  $effect(() => {
    dialog?.showModal();
  });

  function shut(): void {
    dialog?.close();
    onclose();
  }

  /** "61 min · disco inteiro", or how far a read that stopped got. */
  function covered(row: (typeof rows)[number]): string {
    const whole = row.scan.from_bytes === 0 && row.scan.to_bytes >= row.scan.disk_bytes;
    const reach = whole ? t("ui.tests.surface.whole") : t("ui.tests.hist.read", { share: shareOf(row.scan.to_bytes - row.scan.from_bytes, row.scan.disk_bytes) });
    return `${span(row.run.ended_ms - row.run.started_ms)} · ${reach}`;
  }

  function onkeydown(event: KeyboardEvent): void {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    picked = Math.max(0, Math.min(rows.length - 1, picked + step));
    (dialog?.querySelector(`[data-row="${picked}"]`) as HTMLElement | null)?.focus();
  }
</script>

<dialog
  class="reads"
  bind:this={dialog}
  aria-labelledby="reads-title"
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
    <h2 id="reads-title">{t("ui.tests.reads.title", { name })}</h2>
    <button type="button" class="close" aria-label={t("ui.tests.reads.close")} onclick={shut}>×</button>
  </div>
  <div class="cols">
    <div class="list" role="listbox" aria-label={t("ui.tests.reads.list")} tabindex="-1" {onkeydown}>
      {#each rows as row, i (row.run.id)}
        <button
          type="button"
          class="row"
          role="option"
          aria-selected={i === picked}
          data-row={i}
          tabindex={i === picked ? 0 : -1}
          onclick={() => (picked = i)}
        >
          <ConcernMark mark={row.word.mark} word={false} />
          <span>
            <b>{at(row.run.ended_ms)}</b>
            <small>{row.word.text}</small>
            <small>{covered(row)}</small>
          </span>
        </button>
      {/each}
    </div>
    {#if current}
      <div class="detail">
        <p class="res">
          <ConcernMark mark={current.word.mark} word={false} /><b>{current.word.text}</b>
          <span class="when">{at(current.run.ended_ms)} · {covered(current)}</span>
        </p>
        <DiskMap scan={current.scan} diskBytes={current.scan.disk_bytes} {name} {level} />
      </div>
    {/if}
  </div>
</dialog>

<style>
  .reads {
    width: min(900px, calc(100vw - 48px));
    max-height: min(640px, calc(100vh - 80px));
    padding: 16px 18px 18px;
    border: 1px solid var(--stroke);
    border-radius: 14px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .reads[open] {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .reads::backdrop {
    background: #0007;
  }
  .bar {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  h2 {
    margin: 0 auto 0 0;
    font: 600 18px/1.3 var(--fd);
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
  .cols {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    gap: 18px;
    min-height: 0;
  }
  .list {
    display: grid;
    align-content: start;
    gap: 2px;
    overflow: auto;
    outline: none;
  }
  .row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    align-items: start;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: none;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--card2);
  }
  .row[aria-selected="true"] {
    border-color: var(--stroke);
    background: var(--card2);
  }
  .row b {
    display: block;
    font-weight: 600;
    font-size: 14px;
  }
  .row small {
    display: block;
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.4;
  }
  .detail {
    display: grid;
    align-content: start;
    gap: 10px;
    min-width: 0;
    overflow: auto;
  }
  .res {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 8px;
    align-items: center;
    margin: 0;
  }
  .res b {
    font-weight: 600;
  }
  .when {
    color: var(--ink2);
    font-size: 13px;
  }
  @container content (max-width: 640px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (forced-colors: active) {
    .row[aria-selected="true"] {
      border-color: Highlight;
    }
  }
</style>
