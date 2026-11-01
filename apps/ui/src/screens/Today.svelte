<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import { marks, timeline } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatTime } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { buildDay, type Day, dayStart, rangeStart } from "./today/day";
  import Days from "./today/Days.svelte";
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

<h1>{t("ui.today.title")}</h1>
{#if firstUse}
  <p class="lede">
    {startedAt !== null
      ? t("ui.today.first_use", { time: formatTime(startedAt) })
      : t("ui.today.first_use.idle")}
  </p>
  <div class="actions">
    <a class="btn accent" href={href.now()}>{t("ui.today.see_now")}</a>
  </div>
{:else if day}
  <p class="lede">
    {#each lede(day, level) as part, i (i)}{#if part.bold}<b>{part.text}</b>{:else}{part.text}{/if}{/each}
  </p>
  <States {day} {level} />
  <div class="split">
    <Moments items={day.items} {level} />
    <Days days={day.days} />
  </div>
{:else if error !== null}
  <ErrorView {error} onretry={retry} />
{:else}
  <Loading />
{/if}

<style>
  .actions {
    display: flex;
    gap: 10px;
  }
</style>
