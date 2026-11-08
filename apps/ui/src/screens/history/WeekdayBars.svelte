<script lang="ts">
  import { lang, t, tn } from "../../lib/i18n.svelte";

  /** `counts[0]` is Sunday. */
  let { counts }: { counts: number[] } = $props();

  // The 90 px bar area holds the count (16 px), a 3 px gap and a bar of 4 to 70 px.
  const BAR_MIN = 4;
  const BAR_RANGE = 66;

  const top = $derived(Math.max(5, ...counts));
  const letter = $derived(new Intl.DateTimeFormat(lang(), { weekday: "narrow" }));
  const name = $derived(new Intl.DateTimeFormat(lang(), { weekday: "long" }));
  const day = (i: number) => new Date(2024, 0, 7 + i);
  const label = (i: number) => `${name.format(day(i))}: ${counts[i] ? tn("ui.history.count", counts[i]) : t("ui.history.count.zero")}`;
</script>

<div class="week">
  {#each counts as count, i (i)}
    <span class="day" role="img" aria-label={label(i)}>
      <span class="bars" aria-hidden="true">
        <span class="n">{count || ""}</span>
        <i style="height: {BAR_MIN + Math.round((Math.min(count, top) / top) * BAR_RANGE)}px"></i>
      </span>
      <span aria-hidden="true">{letter.format(day(i))}</span>
    </span>
  {/each}
</div>

<style>
  .week {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 8px;
    max-width: 560px;
  }
  .day {
    display: grid;
    grid-template-rows: 90px 14px;
    gap: 4px;
    justify-items: center;
    color: var(--ink3);
    font: 400 11px/14px var(--f);
    text-transform: uppercase;
  }
  .bars {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 3px;
    width: 100%;
    height: 90px;
  }
  .n {
    color: var(--ink);
    font: 400 12px/16px var(--f);
  }
  i {
    display: block;
    width: 100%;
    border-radius: 6px;
    background: var(--calm);
    opacity: 0.75;
  }
  @media (forced-colors: active) {
    i {
      forced-color-adjust: none;
      background: CanvasText;
      opacity: 1;
    }
  }
</style>
