<script lang="ts">
  import { untrack } from "svelte";

  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import SelectField from "../components/SelectField.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { lang, t, tn } from "../lib/i18n.svelte";
  import { type HistoryScope, href } from "../lib/router.svelte";
  import Calendar from "./history/Calendar.svelte";
  import {
    buildHistory,
    clampMonth,
    dayMs,
    daysOf,
    type HistoryData,
    monthOf,
    rangeFor,
    weekday,
  } from "./history/data";
  import WeekdayBars from "./history/WeekdayBars.svelte";
  import { dayStart } from "./today/day";
  import OccurrenceRow from "./today/OccurrenceRow.svelte";

  let {
    month: askedMonth,
    scope,
    day: askedDay,
  }: { month: string | null; scope: HistoryScope; day: string | null } = $props();

  const REFRESH_MS = 5 * 60_000;
  const DEFAULT_KEEP_DAYS = 365;

  let data = $state.raw<HistoryData | null>(null);
  let error = $state<unknown>(null);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const keepDays = $derived(app.settings?.settings.retention.records_days ?? DEFAULT_KEEP_DAYS);
  const firstUse = $derived(app.history === "empty");

  /** Reads every mark kept, so the calendar can go back to the first record. */
  async function load(): Promise<void> {
    const now = Date.now();
    try {
      data = buildHistory(now, await marks(dayStart(now, -keepDays), now));
      error = null;
    } catch (err) {
      if (data === null) error = err;
    }
  }

  $effect(() => {
    if (app.history !== "ready") return;
    untrack(() => void load());
    const timer = setInterval(() => {
      if (!document.hidden) void load();
    }, REFRESH_MS);
    return () => clearInterval(timer);
  });

  const month = $derived(data ? clampMonth(data, askedMonth) : null);
  const range = $derived(data && month ? rangeFor(data, month, scope) : null);
  const days = $derived(range ? daysOf(range) : []);
  const countOf = (key: string) => data?.counts.get(key) ?? 0;
  const selected = $derived(
    data && askedDay && monthOf(askedDay) === month && askedDay >= data.first && askedDay <= data.today ? askedDay : null,
  );

  const total = $derived(days.reduce((sum, d) => sum + countOf(d), 0));
  const worst = $derived(days.reduce<string | null>((best, d) => (best === null || countOf(d) > countOf(best) ? d : best), null));
  const perWeekday = $derived(
    Array.from({ length: 7 }, (_, i) => days.filter((d) => weekday(d) === i).reduce((sum, d) => sum + countOf(d), 0)),
  );

  const longDay = $derived(new Intl.DateTimeFormat(lang(), { weekday: "long", day: "numeric", month: "long" }));
  const nameOf = (key: string) => {
    const text = longDay.format(dayMs(key));
    return text.charAt(0).toLocaleUpperCase(lang()) + text.slice(1);
  };
  const countText = (n: number) => (n ? tn("ui.history.count", n) : t("ui.history.count.zero"));
  const average = $derived(
    new Intl.NumberFormat(lang(), { maximumFractionDigits: 1 }).format(days.length ? total / days.length : 0),
  );
  const summary = $derived(
    [
      t("ui.history.summary", { count: countText(total), days: tn("ui.history.days", days.length) }),
      t("ui.history.average", { avg: average }),
      total && worst ? t("ui.history.worst", { day: nameOf(worst).toLocaleLowerCase(lang()), count: countOf(worst) }) : "",
    ]
      .filter(Boolean)
      .join(" · "),
  );

  const scopes = $derived([
    { value: "month", text: t("ui.history.scope.month") },
    { value: "week", text: t("ui.history.scope.week") },
    { value: "2w", text: t("ui.history.scope.fortnight") },
  ]);

  /** The address holds the month, period and day, so Back from a report lands on the same view. */
  function show(next: { month?: string | null; scope?: HistoryScope; day?: string | null }): void {
    location.replace(
      href.history({
        month: next.month === undefined ? month : next.month,
        scope: next.scope ?? scope,
        day: next.day === undefined ? selected : next.day,
      }),
    );
  }
</script>

<header class="head">
  <h1>{t("ui.item.history")}</h1>
</header>

{#if firstUse}
  <p class="lede">{t("ui.history.first_use")}</p>
{:else if data && month && range}
  <div class="filter">
    <label class="period">
      <span>{t("ui.history.period")}</span>
      <SelectField
        value={scope}
        options={scopes}
        label={t("ui.history.period")}
        onchange={(value) => show({ scope: value as HistoryScope, day: null })}
      />
    </label>
    <p class="summary">{summary}</p>
  </div>
  <div class="hx">
    <div class="left">
      <Block title={t("ui.history.calendar")} tight>
        <Calendar
          {month}
          {data}
          {range}
          {selected}
          onpick={(day) => show({ month: monthOf(day), day })}
          onmonth={(next) => show({ month: next, day: null })}
        />
      </Block>
      <Block title={t("ui.history.weekdays")} tight>
        <WeekdayBars counts={perWeekday} />
      </Block>
    </div>
    <Block title={t("ui.history.daily")} tight fill>
      <div class="wrap">
        <div class="box">
          {#if selected}
            <div class="day-head">
              <p>
                <strong>{nameOf(selected)}</strong>{selected === data.today ? ` (${t("ui.history.today_tag")})` : ""} · {countText(countOf(selected))}
              </p>
              <button type="button" class="btn" onclick={() => show({ day: null })}>{t("ui.history.show_all")}</button>
            </div>
            <div class="scroll">
              {#if countOf(selected)}
                <ul class="events">
                  {#each data.byDay.get(selected) ?? [] as o (o.id)}
                    <li><OccurrenceRow {o} {level} fromDay={selected} /></li>
                  {/each}
                </ul>
              {:else}
                <p class="empty">{t("ui.history.day.none")}</p>
              {/if}
            </div>
          {:else}
            <div class="scroll">
              <RowList>
                {#each days as d (d)}
                  <RowCard
                    onclick={() => show({ day: d })}
                    title={nameOf(d) + (d === data.today ? ` (${t("ui.history.today_tag")})` : "")}
                  >
                    {#snippet right()}{countOf(d) ? countText(countOf(d)) : t("ui.history.none")}{/snippet}
                  </RowCard>
                {/each}
              </RowList>
            </div>
          {/if}
        </div>
      </div>
    </Block>
  </div>
{:else if error !== null}
  <ErrorView {error} onretry={() => void load()} />
{:else}
  <Loading />
{/if}

<style>
  .head {
    margin-bottom: 4px;
  }
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
  .filter {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 24px;
    margin: 14px 0 0;
  }
  .period {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
  }
  .summary {
    margin: 0;
    color: var(--ink2);
    font-size: 13.5px;
  }
  .hx {
    display: grid;
    gap: 0 56px;
  }
  @container content (min-width: 900px) {
    .hx {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      align-items: stretch;
    }
    .wrap {
      flex: 1;
      height: auto;
    }
  }
  .wrap {
    position: relative;
    height: 360px;
  }
  .box {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    scrollbar-gutter: stable;
  }
  .day-head {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 16px;
    margin-bottom: 12px;
  }
  .day-head p {
    margin: 0;
  }
  .events {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .empty {
    margin: 0;
    color: var(--ink2);
  }
</style>
