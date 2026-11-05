<script lang="ts">
  import { t } from "../../lib/i18n.svelte";
  import type { DetailLevel, Region, SurfaceScan } from "../../lib/types";
  import { formatBytes } from "../occurrence/format";
  import { formatNumber } from "../../lib/format";
  import { mapView } from "./mapView.svelte";

  // The disk in 1024 regions, in disk order: what did not read stands out; response time is opt-in.

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

  const uid = $props.id();
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

  const speed = $derived(mapView.speed && scan !== null);
  const cells = $derived<Cell[]>(
    scan ? scan.regions.map(cellOf) : Array.from({ length: 1024 }, (_, i) => (read !== null && i < read * 1024 ? "r" : "x")),
  );
  /** What each cell is drawn as: a read region is plain "r" unless response time is on. */
  const shown = $derived(speed ? cells : cells.map((c) => (typeof c === "number" && c < 6 ? "r" : c)));
  const counts = $derived(scan ? [...scan.bands, scan.unreadable_blocks] : null);

  let at = $state(-1);
  let grid = $state<HTMLDivElement>();

  function bandText(cell: Cell): string {
    if (cell === "x") return t("ui.tests.map.unread");
    if (cell === 6) return t("ui.tests.map.unreadable");
    if (cell === "r" || !speed) return t("ui.tests.map.read");
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
    const worst = cells.findIndex((c) => c === 6 || (speed && c === 5));
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
  ];
</script>

<div class="dmap">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="grid"
    class:speed
    bind:this={grid}
    tabindex="0"
    role="application"
    aria-label={speed
      ? t("ui.tests.map.label", {
          name,
          unreadable: formatNumber(scan?.unreadable_blocks ?? 0),
          slow: formatNumber(scan?.bands[5] ?? 0),
        })
      : t("ui.tests.map.label_plain", { name, unreadable: formatNumber(scan?.unreadable_blocks ?? 0) })}
    aria-describedby="map-readout-{uid}"
    {onkeydown}
    {onfocus}
    onpointermove={(event) => {
      const i = (event.target as HTMLElement).dataset.i;
      if (i !== undefined) at = Number(i);
    }}
    onpointerleave={() => (at = -1)}
  >
    {#each shown as cell, i (i)}
      <i class="c{cell}" data-i={i}></i>
    {/each}
    {#if cursor}
      <b class="cur" style:left="{cursor.left - 2}px" style:top="{cursor.top - 2}px" style:width="{cursor.size + 4}px" style:height="{cursor.size + 4}px"></b>
    {/if}
  </div>
  <p class="readout" id="map-readout-{uid}" aria-live="polite">{readout}</p>
  <div class="foot">
    <div class="legend" class:speed>
      {#if !speed}
        <span><i class="cr"></i>{t("ui.tests.map.read_short")}</span>
      {:else if level === "Simple" || !counts}
        {#each SIMPLE as [cell, key] (key)}
          <span><i class="c{cell}"></i>{t(key)}</span>
        {/each}
      {:else}
        {#each [0, 1, 2, 3, 4, 5] as b (b)}
          <span><i class="c{b}"></i>{t(`ui.tests.band.${b}`)} <b>{formatNumber(counts[b] ?? 0)}</b></span>
        {/each}
      {/if}
      <span>
        <i class="c6"></i>{t("ui.tests.map.unreadable_short")}
        {#if level !== "Simple" && counts}<b>{formatNumber(counts[6] ?? 0)}</b>{/if}
      </span>
      <span><i class="cx"></i>{t("ui.tests.map.unread_short")}</span>
    </div>
    {#if scan}
      <label class="opt">
        <input type="checkbox" class="pick" checked={mapView.speed} onchange={(event) => mapView.setSpeed(event.currentTarget.checked)} />
        {t("ui.tests.map.speed")}
      </label>
    {/if}
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
  .cr {
    background: color-mix(in srgb, var(--ink) 30%, transparent) !important;
  }
  /* Response time, from quick to slow, on one neutral ramp: only what does not read is red. */
  .c0 {
    background: color-mix(in srgb, var(--ink) 14%, transparent) !important;
  }
  .c1 {
    background: color-mix(in srgb, var(--ink) 26%, transparent) !important;
  }
  .c2 {
    background: color-mix(in srgb, var(--ink) 40%, transparent) !important;
  }
  .c3 {
    background: color-mix(in srgb, var(--ink) 56%, transparent) !important;
  }
  .c4 {
    background: color-mix(in srgb, var(--ink) 72%, transparent) !important;
  }
  .c5 {
    background: color-mix(in srgb, var(--ink) 90%, transparent) !important;
  }
  .c6 {
    background: var(--freeze) !important;
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
  .readout {
    min-height: 19px;
    margin: 0;
    color: var(--ink2);
    font-size: 13px;
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    align-items: center;
    justify-content: space-between;
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
  .opt {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    color: var(--ink2);
    font-size: 13px;
    cursor: pointer;
  }
  .pick {
    display: grid;
    flex: none;
    place-items: center;
    width: 18px;
    height: 18px;
    margin: 0;
    border: 1.5px solid var(--ink3);
    border-radius: 5px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .pick:checked {
    border-color: var(--accent);
    background: var(--accent);
  }
  .pick:checked::after {
    content: "";
    width: 8px;
    height: 4px;
    border-bottom: 2px solid var(--on-accent);
    border-left: 2px solid var(--on-accent);
    transform: rotate(-45deg) translate(1px, -1px);
  }
  @media (forced-colors: active) {
    .grid i {
      forced-color-adjust: none;
    }
    .cur {
      box-shadow: 0 0 0 2px Highlight;
    }
    .pick:checked {
      forced-color-adjust: none;
      background: Highlight;
      border-color: Highlight;
    }
  }
</style>
