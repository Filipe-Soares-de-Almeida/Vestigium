<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import { marks, timeline } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatTime } from "../lib/format";
  import { lang, t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { buildDay, type Day, dayStart, rangeStart } from "./today/day";
  import Days from "./today/Days.svelte";
  import HealthCard from "./today/HealthCard.svelte";
  import Moments from "./today/Moments.svelte";
  import States from "./today/States.svelte";
  import { lede } from "./today/text";

  const REFRESH_MS = 60_000;
  /** One point a minute for a whole day. */
  const MAX_POINTS = 24 * 60;

  let day = $state.raw<Day | null>(null);
  let empty = $state(false);
  let error = $state<unknown>(null);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const firstUse = $derived(app.history === "empty" || empty);
  const date = $derived(
    new Intl.DateTimeFormat(lang(), { weekday: "long", day: "numeric", month: "long" }).format(day?.start ?? Date.now()),
  );
  const startedAt = $derived(app.link?.link === "running" ? app.link.started_at_ms : null);

  /** Reads today and the last 14 days; a failed refresh keeps what is on screen. */
  async function load(): Promise<void> {
    const now = Date.now();
    try {
      const [line, found] = await Promise.all([
        timeline(dayStart(now), now, MAX_POINTS),
        marks(rangeStart(now), now),
      ]);
      empty = line.history === "empty";
      day = buildDay(now, line, found);
      error = null;
    } catch (err) {
      if (day === null) error = err;
    }
  }

  function retry(): void {
    error = null;
    void load();
  }

  // Loads once history exists, then again every minute while the window is visible.
  $effect(() => {
    if (app.history !== "ready") return;
    untrack(() => void load());
    const timer = setInterval(() => {
      if (!document.hidden) void load();
    }, REFRESH_MS);
    const onvisible = () => {
      if (!document.hidden && day && Date.now() - day.now >= REFRESH_MS) void load();
    };
    document.addEventListener("visibilitychange", onvisible);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onvisible);
    };
  });
</script>

<header class="head">
  <h1>{t("ui.item.today")}</h1>
  <p class="date">{date}</p>
</header>

{#if firstUse}
  <p class="lede">
    {startedAt !== null
      ? t("ui.today.first_use", { time: formatTime(startedAt) })
      : t("ui.today.first_use.idle")}
  </p>
  <div class="actions">
    <a class="btn accent" href={href.activity("now")}>{t("ui.today.see_now")}</a>
  </div>
{:else if day}
  <p class="lede">
    {#each lede(day, level) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
  </p>
  {#if app.suggestCapture}
    <p class="suggest">
      <Icon name="captures" />
      <span>{t("ui.captures.suggest.text")} <a href={href.captures()}>{t("ui.captures.suggest.link")}</a></span>
    </p>
  {/if}
  <States {day} {level} />
  <div class="cols">
    <Moments items={day.items} {level} />
    <HealthCard />
  </div>
  <Days days={day.days} />
{:else if error !== null}
  <ErrorView {error} onretry={retry} />
{:else}
  <Loading />
{/if}

<style>
  .head {
    margin-bottom: 4px;
  }
  .date {
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0 56px;
  }
  @container content (min-width: 1000px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      align-items: start;
    }
  }
  .actions {
    display: flex;
    gap: 10px;
  }
  .suggest {
    display: flex;
    gap: 10px;
    align-items: start;
    margin: -8px 0 14px;
    color: var(--ink2);
    font-size: 13.5px;
    line-height: 1.45;
  }
  .suggest :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .suggest a {
    font-weight: 600;
    text-decoration: none;
    border-radius: 6px;
  }
  .suggest a:hover {
    text-decoration: underline;
  }
</style>
