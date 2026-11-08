<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { href } from "../lib/router.svelte";
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

  relatedLinks(() => {
    const last = view.frame?.latest ?? null;
    return [
      { label: t("ui.related.all_programs"), href: href.programs() },
      ...(last ? [{ label: t("ui.related.last_freeze"), href: href.occurrence(last.id) }] : []),
      { label: t("ui.item.today"), href: href.activity() },
    ];
  });

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

{#if !stream.watching}
  <PageHead
    title={t("ui.now.title")}
    sub={t(app.paused ? "ui.now.lede.paused" : app.stopped ? "ui.now.lede.stopped" : "ui.now.lede.unreachable")}
  />
{:else if stream.error !== null}
  <PageHead title={t("ui.now.title")} />
  <ErrorView error={stream.error} onretry={() => stream.retry()} />
{:else if !live.started}
  <PageHead title={t("ui.now.title")} />
  <Loading />
{:else}
  <PageHead title={t("ui.now.title")} sub={lede} />
  <TenMinutes points={view.points} paused={stream.paused} onsay={(text) => (said = text)} />
  <p class="sr" aria-live="polite">{said}</p>
  <div class="cols2">
    <TopPrograms top={view.frame?.top ?? null} programs={view.frame?.programs ?? null} />
    <div>
      <HungWindows hung={view.hung} nowMs={view.nowMs} />
      <LastOccurrence latest={view.frame?.latest ?? null} {since} />
    </div>
  </div>
{/if}
