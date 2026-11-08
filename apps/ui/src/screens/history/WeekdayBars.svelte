<script lang="ts">
  import { lang, t, tn } from "../../lib/i18n.svelte";

  /** `counts[0]` is Sunday. */
  let { counts }: { counts: number[] } = $props();

  // Bars run from 4 px to 86 px on the grid of the 14-day chart, the busiest weekday at the top.
  const BAR_MIN = 4;
  const BAR_RANGE = 86;

  const top = $derived(Math.max(1, ...counts));
  const letter = $derived(new Intl.DateTimeFormat(lang(), { weekday: "narrow" }));
  const name = $derived(new Intl.DateTimeFormat(lang(), { weekday: "long" }));
  const day = (i: number) => new Date(2024, 0, 7 + i);
  const label = (i: number) => `${name.format(day(i))}: ${counts[i] ? tn("ui.history.count", counts[i]) : t("ui.history.count.zero")}`;
</script>

<div class="week">
  {#each counts as count, i (i)}
    <span class="day" role="img" aria-label={label(i)}>
      <span class="n" aria-hidden="true">{count || ""}</span>
      <i aria-hidden="true" style="height: {Math.max(BAR_MIN, (count / top) * BAR_RANGE)}px"></i>
      <span class="l" aria-hidden="true">{letter.format(day(i))}</span>
    </span>
  {/each}
</div>

<style>
  .week {
    display: grid;
    grid-template-columns: repeat(14, minmax(0, 1fr));
    gap: 5px;
    align-items: end;
    max-width: 760px;
  }
  .day {
    display: grid;
    justify-items: center;
    gap: 4px;
  }
  .n {
    min-height: 16px;
    color: var(--ink2);
    font-size: 12px;
    line-height: 16px;
  }
  i {
    display: block;
    width: 100%;
    border-radius: 6px;
    background: var(--calm);
    opacity: 0.75;
  }
  .l {
    color: var(--ink3);
    font-size: 11px;
    line-height: 18px;
    text-transform: uppercase;
  }
  @media (forced-colors: active) {
    i {
      forced-color-adjust: none;
      background: CanvasText;
      opacity: 1;
    }
  }
</style>
