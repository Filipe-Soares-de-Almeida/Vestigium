<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import SelectField from "../components/SelectField.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { marks } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatDuration, formatTime } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { href } from "../lib/router.svelte";
  import { dayStart, type Item, itemsOf } from "./today/day";
  import { causeName, episodeDisk, episodeKey, episodeTitle, occurrenceMark, occurrenceTitle } from "./today/text";

  const REFRESH_MS = 60_000;
  type Filter = "all" | "freeze" | "stutter" | "hang" | "period";

  let items = $state.raw<Item[] | null>(null);
  let error = $state<unknown>(null);
  let filter = $state("all");

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const firstUse = $derived(app.history === "empty");

  async function load(): Promise<void> {
    const now = Date.now();
    const start = dayStart(now);
    try {
      items = itemsOf(start, now, await marks(start, now)).items;
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
          from: formatTime(items[0].start),
          to: formatTime(items[items.length - 1].start),
        })
      : "",
  );

  /** One table row: when, what, which kind, where from, how long and how grave. */
  function rowOf(item: Item) {
    if (item.kind === "occurrence") {
      const o = item.occurrence;
      return {
        at: o.kind === "PostMortem" ? o.end_ms : o.start_ms,
        title: occurrenceTitle(o, level),
        href: href.occurrence(o.id),
        type: t(`ui.common.kind.${o.kind}`),
        origin: causeName(o.cause),
        duration: o.kind === "PostMortem" ? "—" : formatDuration(o.end_ms - o.start_ms),
        mark: occurrenceMark(o),
      };
    }
    const e = item.episode;
    const key = episodeKey(e);
    return {
      at: e.start_ms,
      title: episodeTitle(e, level),
      href: href.period(e.start_ms),
      type: t("ui.today.episode.kind"),
      origin:
        e.culprit?.name ??
        (key === "DiskBusy" ? episodeDisk(e) : t(key === "MemoryHigh" ? "ui.now.lane.memory" : "ui.now.lane.cpu")),
      duration: formatDuration(e.end_ms - e.start_ms),
      mark: { kind: "overload" as const, word: t("ui.status.overload") },
    };
  }

  relatedLinks(() => [
    { label: t("ui.related.states_day"), href: href.activity() },
    { label: t("ui.related.capture"), href: href.captures() },
    { label: t("ui.item.history"), href: href.history() },
  ]);
</script>

<PageHead title={t("ui.item.events")} {sub} />

{#if firstUse}
  <p class="lede">{t("ui.events.first_use")}</p>
{:else if items}
  <div class="tools">
    <label class="inl">
      <span>{t("ui.events.show")}</span>
      <SelectField bind:value={filter} options={filters} label={t("ui.events.show")} />
    </label>
  </div>
  {#if shown.length}
    <div class="wide">
      <table class="tbl tbl-ev">
        <thead>
          <tr>
            <th scope="col">{t("ui.events.col.time")}</th>
            <th scope="col">{t("ui.events.col.event")}</th>
            <th scope="col">{t("ui.events.col.type")}</th>
            <th scope="col">{t("ui.events.col.origin")}</th>
            <th scope="col">{t("ui.events.col.duration")}</th>
            <th scope="col">{t("ui.events.col.gravity")}</th>
          </tr>
        </thead>
        <tbody>
          {#each shown as item (`${item.kind}-${item.start}`)}
            {@const row = rowOf(item)}
            <tr>
              <td>{formatTime(row.at)}</td>
              <td class="nm" title={row.title}><a class="row-a" href={row.href}>{row.title}</a></td>
              <td>{row.type}</td>
              <td>{row.origin}</td>
              <td>{row.duration}</td>
              <td><StatusMark kind={row.mark.kind} word={row.mark.word} /></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="empty">{t(filter === "all" ? "ui.today.moments.none" : "ui.events.none")}</p>
  {/if}
{:else if error !== null}
  <ErrorView {error} onretry={() => void load()} />
{:else}
  <Loading />
{/if}
