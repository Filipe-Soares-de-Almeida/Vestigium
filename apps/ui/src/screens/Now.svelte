<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import HungWindows from "./now/HungWindows.svelte";
  import LastOccurrence from "./now/LastOccurrence.svelte";
  import { momentKey, sentence } from "./now/sentence";
  import { NowStream } from "./now/stream.svelte";
  import TenMinutes from "./now/TenMinutes.svelte";
  import TopPrograms from "./now/TopPrograms.svelte";

  const stream = new NowStream();
  const view = $derived(stream.view);
  const live = stream.live;
  /** What the polite live region last said. */
  let said = $state("");

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const moment = $derived(
    view.point && view.frame
      ? { point: view.point, frame: view.frame, hung: view.hung, nowMs: view.nowMs }
      : null,
  );
  const lede = $derived(moment ? sentence(moment, level) : t("ui.now.lede.waiting"));
  const key = $derived(moment ? momentKey(moment) : "");
  const since = $derived(
    app.history === "empty" && app.link?.link === "running" ? app.link.started_at_ms : null,
  );

  // The region speaks the answer only when the state of the PC changes, never every second.
  let spoken = "";
  $effect(() => {
    const now = key;
    if (now === spoken) return;
    const first = spoken === "";
    spoken = now;
    if (!first && now) said = untrack(() => lede);
  });
</script>

<header class="head">
  <h1>{t("ui.item.live")}</h1>
</header>

{#if !stream.watching}
  <p class="lede">
    {t(app.paused ? "ui.now.lede.paused" : app.stopped ? "ui.now.lede.stopped" : "ui.now.lede.unreachable")}
  </p>
{:else if stream.error !== null}
  <ErrorView error={stream.error} onretry={() => stream.retry()} />
{:else if !live.started}
  <Loading />
{:else}
  <p class="lede">{lede}</p>
  <p class="sr" aria-live="polite">{said}</p>
  <TenMinutes points={view.points} paused={stream.paused} onsay={(text) => (said = text)} />
  <div class="cols">
    <TopPrograms top={view.frame?.top ?? null} total={view.frame?.programs?.entries.length ?? 0} />
    <div>
      <HungWindows hung={view.hung} nowMs={view.nowMs} />
      <LastOccurrence latest={view.frame?.latest ?? null} {since} />
    </div>
  </div>
{/if}

<style>
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
</style>
