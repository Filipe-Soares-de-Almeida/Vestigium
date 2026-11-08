<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import SelectField from "../components/SelectField.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatTime } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import { dayStart, type Item, itemsOf } from "./today/day";
  import EpisodeRow from "./today/EpisodeRow.svelte";
  import OccurrenceRow from "./today/OccurrenceRow.svelte";

  const REFRESH_MS = 60_000;
  type Filter = "all" | "freeze" | "stutter" | "hang" | "period";

  let items = $state.raw<Item[] | null>(null);
  let range = $state<[number, number]>([0, 0]);
  let error = $state<unknown>(null);
  let filter = $state("all");

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const firstUse = $derived(app.history === "empty");

  async function load(): Promise<void> {
    const now = Date.now();
    const start = dayStart(now);
    try {
      items = itemsOf(start, now, await marks(start, now)).items;
      range = [start, now];
      error = null;
    } catch (err) {
      if (items === null) error = err;
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

  const KIND: Record<Exclude<Filter, "all" | "period">, string[]> = {
    freeze: ["Freeze", "PostMortem"],
    stutter: ["Stutter"],
    hang: ["AppHang"],
  };
  const matches = (item: Item) => {
    const wanted = filter as Filter;
    if (wanted === "all") return true;
    if (item.kind === "episode") return wanted === "period";
    return wanted !== "period" && KIND[wanted].includes(item.occurrence.kind);
  };

  const shown = $derived((items ?? []).filter(matches));
  const filters = $derived([
    { value: "all", text: t("ui.events.filter.all") },
    { value: "freeze", text: t("ui.events.filter.freeze") },
    { value: "stutter", text: t("ui.events.filter.stutter") },
    { value: "hang", text: t("ui.events.filter.hang") },
    { value: "period", text: t("ui.events.filter.period") },
  ]);
  const occurrences = $derived((items ?? []).filter((i) => i.kind === "occurrence").length);
  const periods = $derived((items ?? []).length - occurrences);
  const sub = $derived(
    items && items.length
      ? t("ui.events.sub", {
          occurrences: occurrences ? tn("ui.events.occurrences", occurrences) : t("ui.events.occurrences.none"),
          periods: periods ? tn("ui.events.periods", periods) : t("ui.events.periods.none"),
          from: formatTime(range[0]),
          to: formatTime(range[1]),
        })
      : "",
  );
</script>

<header class="head">
  <h1>{t("ui.item.events")}</h1>
  {#if sub}<p class="sub">{sub}</p>{/if}
</header>

{#if firstUse}
  <p class="lede">{t("ui.events.first_use")}</p>
{:else if items}
  <div class="tools">
    <label class="show">
      <span>{t("ui.events.show")}</span>
      <SelectField bind:value={filter} options={filters} label={t("ui.events.show")} />
    </label>
  </div>
  {#if shown.length}
    <ul>
      {#each shown as item (`${item.kind}-${item.start}`)}
        <li>
          {#if item.kind === "occurrence"}
            <OccurrenceRow o={item.occurrence} {level} />
          {:else}
            <EpisodeRow e={item.episode} {level} byTest={item.byTest} />
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">{t(filter === "all" ? "ui.today.moments.none" : "ui.events.none")}</p>
  {/if}
{:else if error !== null}
  <ErrorView {error} onretry={() => void load()} />
{:else}
  <Loading />
{/if}

<style>
  .head {
    margin-bottom: 4px;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
  .tools {
    margin: 16px 0 12px;
  }
  .show {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
  }
  ul {
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
