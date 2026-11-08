<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { lang, t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DayCount } from "./day";

  let { days }: { days: DayCount[] } = $props();

  // Bars run from 4 px to 86 px, the tallest day at the top.
  const BAR_MIN = 4;
  const BAR_RANGE = 86;

  const top = $derived(Math.max(1, ...days.map((d) => d.count)));
  const total = $derived(days.reduce((sum, d) => sum + d.count, 0));
  const letter = $derived(new Intl.DateTimeFormat(lang(), { weekday: "narrow" }));
  const spelled = $derived(
    new Intl.DateTimeFormat(lang(), { weekday: "long", day: "numeric", month: "long" }),
  );

  /** "Sunday, October 25: 1 freeze", the whole bar as one labelled image. */
  function label(d: DayCount): string {
    const day = spelled.format(d.start);
    return d.count === 0 ? t("ui.today.days.none", { day }) : tn("ui.today.days.count", d.count, { day });
  }
</script>

<Block title={t("ui.today.days.title")}>
  <div class="days">
    {#each days as d (d.start)}
      <span class="day" class:today={d.today} role="img" aria-label={label(d)} aria-current={d.today ? "date" : undefined}>
        <span class="n" aria-hidden="true">{d.count || ""}</span>
        <i aria-hidden="true" style="height: {Math.max(BAR_MIN, (d.count / top) * BAR_RANGE)}px"></i>
        <span class="l" aria-hidden="true">{letter.format(d.start)}</span>
      </span>
    {/each}
  </div>
  <p class="note">{total === 0 ? t("ui.today.days.summary.none") : tn("ui.today.days.summary", total)}</p>
  <p class="more"><a href={href.history()}>{t("ui.today.days.history")}</a></p>
</Block>

<style>
  .days {
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
  .today i {
    background: var(--freeze);
    opacity: 1;
  }
  .l {
    color: var(--ink3);
    font-size: 11px;
    line-height: 18px;
  }
  .today .l {
    color: var(--ink);
    font-weight: 600;
  }
  @media (forced-colors: active) {
    i {
      forced-color-adjust: none;
      background: CanvasText;
      opacity: 1;
    }
    .today i {
      background: Highlight;
    }
  }
</style>
