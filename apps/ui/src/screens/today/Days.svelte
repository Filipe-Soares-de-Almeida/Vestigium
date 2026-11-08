<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { lang, t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DayCount } from "./day";

  let { days }: { days: DayCount[] } = $props();

  // The 90 px bar area holds the count (16 px), a 3 px gap and a bar of 4 to 70 px.
  const BAR_MIN = 4;
  const BAR_RANGE = 66;

  const top = $derived(Math.max(5, ...days.map((d) => d.count)));
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
  <div class="week">
    {#each days as d (d.start)}
      <span class="day" role="img" aria-label={label(d)} aria-current={d.today ? "date" : undefined}>
        <span class="bars" aria-hidden="true">
          <span class="n">{d.count || ""}</span>
          <i style="height: {BAR_MIN + Math.round((Math.min(d.count, top) / top) * BAR_RANGE)}px"></i>
        </span>
        <span aria-hidden="true">{letter.format(d.start)}</span>
      </span>
    {/each}
  </div>
  <p class="caption">{t("ui.today.days.caption")}</p>
  <p class="more"><a href={href.history()}>{t("ui.today.days.history")}</a></p>
  <p class="sr">
    {total === 0 ? t("ui.today.days.summary.none") : tn("ui.today.days.summary", total)}
  </p>
</Block>

<style>
  .week {
    display: grid;
    grid-template-columns: repeat(14, minmax(0, 1fr));
    gap: 5px;
  }
  .day {
    display: grid;
    grid-template-rows: 90px 14px;
    gap: 4px;
    justify-items: center;
    color: var(--ink3);
    font: 400 11px/14px var(--f);
  }
  .day[aria-current="date"] {
    color: var(--ink);
    font-weight: 600;
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
  .day[aria-current="date"] i {
    background: var(--freeze);
    opacity: 1;
  }
  .more {
    margin: 12px 0 0;
    font-size: 14px;
  }
  .more a {
    font-weight: 600;
  }
  .caption {
    margin: 10px 0 0;
    color: var(--ink2);
    font-size: 12.5px;
  }
  @media (forced-colors: active) {
    i {
      forced-color-adjust: none;
      background: CanvasText;
      opacity: 1;
    }
    .day[aria-current="date"] i {
      background: Highlight;
    }
  }
</style>
