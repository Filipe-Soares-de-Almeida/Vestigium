<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel, Region, SurfaceScan } from "../../lib/types";
  import { formatBytes } from "../occurrence/format";
  import { formatNumber } from "../../lib/format";

  // The disk in 1024 regions, in disk order, each toned by its slowest block; arrows move a cursor.

  let {
    scan = null,
    read = null,
    diskBytes,
    name,
    level,
  }: {
    scan?: SurfaceScan | null;
    /** While a scan runs without a map yet: the share of the disk read so far. */
    read?: number | null;
    diskBytes: number;
    name: string;
    level: DetailLevel;
  } = $props();

  const COLUMNS = 64;
  const BOUNDS = [5, 20, 50, 150, 500];

  /** "x" unread, 0–5 the latency band, 6 a region with sectors that do not read, "r" read during a live scan. */
  type Cell = "x" | "r" | 0 | 1 | 2 | 3 | 4 | 5 | 6;

  function cellOf(region: Region): Cell {
    if (region === "Unread") return "x";
    if (region === "Unreadable") return 6;
    const band = BOUNDS.findIndex((b) => region.Read.worst_ms < b);
    return (band === -1 ? 5 : band) as Cell;
  }

  const cells = $derived<Cell[]>(
    scan ? scan.regions.map(cellOf) : Array.from({ length: 1024 }, (_, i) => (read !== null && i < read * 1024 ? "r" : "x")),
  );
  const counts = $derived(scan ? [...scan.bands, scan.unreadable_blocks] : null);

  let at = $state(-1);
  let grid = $state<HTMLDivElement>();

  function bandText(cell: Cell): string {
    if (cell === "x") return t("ui.tests.map.unread");
    if (cell === "r") return t("ui.tests.map.read");
    if (cell === 6) return t("ui.tests.map.unreadable");
    return t("ui.tests.map.slowest", { band: t(`ui.tests.band.${cell}`) });
  }

  const readout = $derived(
    at < 0
      ? t(scan ? "ui.tests.map.hint" : "ui.tests.map.live")
      : `${t("ui.tests.map.range", {
          from: formatBytes((at / 1024) * diskBytes),
          to: formatBytes(((at + 1) / 1024) * diskBytes),
        })} · ${bandText(cells[at])}`,
  );

  function move(next: number): void {
    at = Math.max(0, Math.min(cells.length - 1, next));
  }

  function onkeydown(event: KeyboardEvent): void {
    const step = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: COLUMNS, ArrowUp: -COLUMNS, Home: -1e4, End: 1e4 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    move((at < 0 ? 0 : at) + step);
  }

  function onfocus(): void {
    if (at >= 0) return;
    const worst = cells.findIndex((c) => c === 6 || c === 5);
    move(worst >= 0 ? worst : 0);
  }

  const cursor = $derived.by(() => {
    if (at < 0 || !grid) return null;
    const cell = grid.children[at] as HTMLElement | undefined;
    return cell ? { left: cell.offsetLeft, top: cell.offsetTop, size: cell.offsetWidth } : null;
  });

  const SIMPLE: [Cell, string][] = [
    [1, "ui.tests.band.simple.normal"],
    [3, "ui.tests.band.simple.slow"],
    [4, "ui.tests.band.simple.slower"],
    [5, "ui.tests.band.simple.slowest"],
    [6, "ui.tests.band.simple.unreadable"],
  ];
</script>

<div class="dmap">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="grid"
    bind:this={grid}
    tabindex="0"
    role="application"
    aria-label={t("ui.tests.map.label", {
      name,
      unreadable: formatNumber(scan?.unreadable_blocks ?? 0),
      slow: formatNumber(scan?.bands[5] ?? 0),
    })}
    aria-describedby="map-readout"
    {onkeydown}
    {onfocus}
    onpointermove={(event) => {
      const i = (event.target as HTMLElement).dataset.i;
      if (i !== undefined) at = Number(i);
    }}
    onpointerleave={() => (at = -1)}
  >
    {#each cells as cell, i (i)}
      <i class="c{cell}" data-i={i}></i>
    {/each}
    {#if cursor}
      <b class="cur" style:left="{cursor.left - 2}px" style:top="{cursor.top - 2}px" style:width="{cursor.size + 4}px" style:height="{cursor.size + 4}px"></b>
    {/if}
  </div>
  <div class="axis" aria-hidden="true">
    <span>{t("ui.tests.map.start")}</span><span>{formatBytes(diskBytes / 2)}</span><span>{t("ui.tests.map.end", { size: formatBytes(diskBytes) })}</span>
  </div>
  <p class="readout" id="map-readout" aria-live="polite">{readout}</p>
  <div class="legend">
    {#if level === "Simple" || !counts}
      {#each SIMPLE as [cell, key] (key)}
        <span><i class="c{cell}"></i>{t(key)}</span>
      {/each}
    {:else}
      {#each [0, 1, 2, 3, 4, 5, 6] as b (b)}
        <span><i class="c{b}"></i>{t(b === 6 ? "ui.tests.map.unreadable_short" : `ui.tests.band.${b}`)} <b>{formatNumber(counts[b] ?? 0)}</b></span>
      {/each}
    {/if}
    <span><i class="cx"></i>{t("ui.tests.map.unread_short")}</span>
  </div>
</div>

<style>
  .dmap {
    display: grid;
    gap: 6px;
  }
  .grid {
    position: relative;
    display: grid;
    grid-template-columns: repeat(64, minmax(0, 1fr));
    gap: 2px;
    padding: 2px;
    border-radius: 8px;
    outline: none;
  }
  .grid i,
  .legend i {
    display: block;
    border-radius: 2px;
    background: var(--card2);
  }
  .grid i {
    aspect-ratio: 1;
  }
  .legend i {
    display: inline-block;
    width: 11px;
    height: 11px;
    border-radius: 2px;
  }
  .c0 {
    background: color-mix(in srgb, var(--ink) 16%, transparent) !important;
  }
  .c1,
  .cr {
    background: color-mix(in srgb, var(--ink) 30%, transparent) !important;
  }
  .c2 {
    background: color-mix(in srgb, var(--ink) 52%, transparent) !important;
  }
  .c3 {
    background: var(--busy) !important;
  }
  .c4 {
    background: var(--press) !important;
  }
  .c5 {
    background: var(--freeze) !important;
  }
  .c6 {
    background: repeating-linear-gradient(45deg, var(--freeze) 0 2px, var(--ink) 2px 3.5px) !important;
  }
  .cx {
    background: transparent !important;
    box-shadow: inset 0 0 0 1px var(--stroke2);
  }
  .cur {
    position: absolute;
    border-radius: 6px;
    box-shadow:
      0 0 0 2px var(--ink),
      0 0 0 4px var(--layer);
    pointer-events: none;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    color: var(--ink3);
    font-size: 12px;
  }
  .readout {
    min-height: 19px;
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--ink2);
    font-size: 12.5px;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .legend b {
    color: var(--ink);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  @media (forced-colors: active) {
    .grid i {
      forced-color-adjust: none;
    }
    .cur {
      box-shadow: 0 0 0 2px Highlight;
    }
  }
</style>
