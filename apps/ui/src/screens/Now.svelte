<script lang="ts">
  import { untrack } from "svelte";

  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import { liveStart } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { keepFocus } from "../lib/focus";
  import { CtrlHold } from "../lib/hold.svelte";
  import { t } from "../lib/i18n.svelte";
  import type { LivePoint, NowFrame, UiError } from "../lib/types";
  import { whileVisible } from "../lib/visibility";
  import HungWindows from "./now/HungWindows.svelte";
  import LastOccurrence from "./now/LastOccurrence.svelte";
  import { Live, type Hung } from "./now/live.svelte";
  import { momentKey, sentence } from "./now/sentence";
  import TenMinutes from "./now/TenMinutes.svelte";
  import TopPrograms from "./now/TopPrograms.svelte";

  const STOPPED: UiError = { kind: "agent_stopped", message: "" };

  const live = new Live();
  const hold = new CtrlHold();
  /** What the screen shows while Ctrl is held; the stream keeps filling `live` underneath. */
  let frozen = $state.raw<{
    points: LivePoint[];
    point: LivePoint | null;
    frame: NowFrame | null;
    hung: Hung[];
    nowMs: number;
  } | null>(null);
  const view = $derived(frozen ?? live);
  let failure = $state<unknown>(null);
  let attempt = $state(0);
  /** What the polite live region last said. */
  let said = $state("");

  const watching = $derived(app.running && !app.paused);
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

  // The live stream runs only while this screen is shown, the window is visible and watching runs.
  $effect(() => {
    if (!watching) return;
    void attempt;
    untrack(() => {
      failure = null;
      live.reset();
    });
    return whileVisible(
      () => liveStart((message) => live.take(message)),
      (error) => (failure = error),
    );
  });

  /** Starts the stream again; focus stays inside the screen while the retry button goes away. */
  function retry(): void {
    const main = document.getElementById("main");
    if (main) void keepFocus(main, () => attempt++);
    else attempt++;
  }

  $effect(() => hold.listen());

  // Holding Ctrl keeps the screen on the second it was pressed; letting go returns to now.
  $effect(() => {
    const held = hold.held;
    untrack(() => {
      frozen =
        held && live.started
          ? { points: live.points, point: live.point, frame: live.frame, hung: live.hung, nowMs: live.nowMs }
          : null;
    });
  });

  // The watcher stopped answering: ask the shell again so its banner shows at once.
  $effect(() => {
    if (live.ended) void untrack(() => app.refresh());
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

<h1>{t("ui.now.title")}</h1>
{#if !watching}
  <p class="lede">
    {t(app.paused ? "ui.now.lede.paused" : app.stopped ? "ui.now.lede.stopped" : "ui.now.lede.unreachable")}
  </p>
{:else if failure !== null || live.ended}
  <ErrorView error={failure ?? STOPPED} onretry={retry} />
{:else if !live.started}
  <Loading />
{:else}
  <p class="lede">{lede}</p>
  <p class="sr" aria-live="polite">{said}</p>
  <TenMinutes points={view.points} paused={frozen !== null} onsay={(text) => (said = text)} />
  <div class="split">
    <TopPrograms top={view.frame?.top ?? null} />
    <div class="stack">
      <HungWindows hung={view.hung} nowMs={view.nowMs} />
      <LastOccurrence latest={view.frame?.latest ?? null} {since} />
    </div>
  </div>
{/if}
