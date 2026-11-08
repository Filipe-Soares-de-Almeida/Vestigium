<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { lang, t, tn } from "../../lib/i18n.svelte";
  import { addDays, dayMs, type HistoryData, monthOf, monthRange, monthStep, weekday } from "./data";

  let {
    month,
    data,
    range,
    selected,
    onpick,
    onmonth,
  }: {
    month: string;
    data: HistoryData;
    range: { first: string; last: string };
    selected: string | null;
    onpick: (day: string) => void;
    onmonth: (month: string) => void;
  } = $props();

  const HEAT = [0, 14, 24, 32, 40];

  const title = $derived.by(() => {
    const text = new Intl.DateTimeFormat(lang(), { month: "long", year: "numeric" }).format(dayMs(`${month}-01`));
    return text.charAt(0).toLocaleUpperCase(lang()) + text.slice(1);
  });
  const initials = $derived.by(() => {
    const format = new Intl.DateTimeFormat(lang(), { weekday: "narrow" });
    const long = new Intl.DateTimeFormat(lang(), { weekday: "long" });
    return Array.from({ length: 7 }, (_, i) => ({
      letter: format.format(new Date(2024, 0, 7 + i)),
      name: long.format(new Date(2024, 0, 7 + i)),
    }));
  });
  const longDay = $derived(new Intl.DateTimeFormat(lang(), { weekday: "long", day: "numeric", month: "long" }));

  const cells = $derived.by(() => {
    const { from, to } = monthRange(month);
    const gridFrom = addDays(from, -weekday(from));
    return Array.from({ length: 42 }, (_, i) => {
      const key = addDays(gridFrom, i);
      const live = key >= from && key <= to && key >= data.first && key <= data.today;
      const count = data.counts.get(key) ?? 0;
      return { key, num: Number(key.slice(8)), live, count, inRange: key >= range.first && key <= range.last };
    });
  });

  const canPrev = $derived(monthStep(month, -1) >= monthOf(data.first));
  const canNext = $derived(monthStep(month, 1) <= monthOf(data.today));

  const countText = (n: number) => (n ? tn("ui.history.count", n) : t("ui.history.count.zero"));
</script>

<div class="bar">
  <button type="button" class="nav prev" aria-label={t("ui.history.prev")} disabled={!canPrev} onclick={() => onmonth(monthStep(month, -1))}>
    <Icon name="chevron" />
  </button>
  <h3 aria-live="polite">{title}</h3>
  <button type="button" class="nav" aria-label={t("ui.history.next")} disabled={!canNext} onclick={() => onmonth(monthStep(month, 1))}>
    <Icon name="chevron" />
  </button>
</div>
<div class="cal" role="group" aria-label={title}>
  {#each initials as day, i (i)}
    <span class="head" title={day.name}>{day.letter}</span>
  {/each}
  {#each cells as cell (cell.key)}
    {#if cell.live}
      <button
        type="button"
        class="day"
        class:today={cell.key === data.today}
        class:out={!cell.inRange}
        style="--heat: {HEAT[Math.min(cell.count, 4)]}"
        aria-pressed={cell.key === selected}
        aria-label="{longDay.format(dayMs(cell.key))}: {countText(cell.count)}"
        onclick={() => onpick(cell.key)}
      >
        <span class="num">{cell.num}{#if cell.key === data.today}<em> · {t("ui.history.today_tag")}</em>{/if}</span>
        <span class="count">{cell.count || ""}</span>
      </button>
    {:else}
      <span class="day off" aria-hidden="true"><span class="num">{cell.num}</span></span>
    {/if}
  {/each}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 560px;
    margin: 0 0 8px;
  }
  h3 {
    flex: 1;
    margin: 0;
    font: 400 15px/20px var(--f);
    text-align: center;
  }
  .nav {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: var(--row);
    color: var(--ink);
    cursor: pointer;
  }
  .nav:hover:not(:disabled) {
    background: var(--row-hover);
  }
  .nav:disabled {
    cursor: default;
    opacity: 0.3;
  }
  .nav:focus-visible,
  .day:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }
  .prev :global(svg) {
    transform: rotate(180deg);
  }
  .cal {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    grid-template-rows: 20px repeat(6, 42px);
    gap: 4px;
    max-width: 560px;
  }
  .head {
    height: 20px;
    color: var(--ink2);
    font-size: 12px;
    line-height: 20px;
    text-align: center;
    text-transform: uppercase;
  }
  .day {
    position: relative;
    display: grid;
    grid-template-rows: auto 1fr;
    height: 42px;
    padding: 3px 7px;
    border: 0;
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) calc(var(--heat, 0) * 1%), var(--row));
    color: var(--ink);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .day:hover {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink3) 55%, transparent);
  }
  .day[aria-pressed="true"] {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .day.out {
    opacity: 0.5;
  }
  .day.off {
    background: none;
    cursor: default;
    opacity: 0.4;
  }
  .num {
    color: var(--ink2);
    font-size: 12px;
  }
  .today .num {
    color: var(--ink);
    font-weight: 700;
  }
  em {
    color: var(--ink2);
    font-style: normal;
    font-weight: 400;
  }
  .count {
    align-self: end;
    justify-self: end;
    font: 600 16px/1 var(--fd);
  }
</style>
