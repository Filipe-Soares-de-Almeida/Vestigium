<script lang="ts">
  import { onMount, tick, untrack } from "svelte";

  import Banner from "./components/Banner.svelte";
  import ErrorView from "./components/ErrorView.svelte";
  import Loading from "./components/Loading.svelte";
  import NavPane from "./components/NavPane.svelte";
  import Related from "./components/Related.svelte";
  import Toast from "./components/Toast.svelte";
  import UpdateDialog from "./components/UpdateDialog.svelte";
  import WindowControls from "./components/WindowControls.svelte";
  import { app } from "./lib/app.svelte";
  import { captures } from "./lib/capture.svelte";
  import { dumps } from "./lib/dumps.svelte";
  import { hwtests } from "./lib/tests.svelte";
  import { tweaks } from "./lib/tweaks.svelte";
  import { markFrame } from "./lib/frame";
  import { health } from "./lib/health.svelte";
  import { router } from "./lib/router.svelte";
  import { today } from "./lib/today.svelte";
  import { updates } from "./lib/update.svelte";
  import Activity from "./screens/Activity.svelte";
  import Capture from "./screens/Capture.svelte";
  import Captures from "./screens/Captures.svelte";
  import Dumps from "./screens/Dumps.svelte";
  import Home from "./screens/Home.svelte";
  import { morph } from "./lib/morph.svelte";
  import Health from "./screens/Health.svelte";
  import History from "./screens/History.svelte";
  import Detail from "./screens/Detail.svelte";
  import Disks from "./screens/Disks.svelte";
  import Events from "./screens/Events.svelte";
  import Memory from "./screens/Memory.svelte";
  import Occurrence from "./screens/Occurrence.svelte";
  import PeriodPage from "./screens/PeriodPage.svelte";
  import Programs from "./screens/Programs.svelte";
  import Settings from "./screens/Settings.svelte";
  import Tests from "./screens/Tests.svelte";
  import Tweaks from "./screens/Tweaks.svelte";

  let content: HTMLElement | undefined = $state();

  let main: HTMLElement | undefined = $state();
  let shown = "";

  onMount(() => app.start());
  onMount(() => captures.watch());
  onMount(() => void updates.start());
  onMount(() => void markFrame());
  onMount(() => health.watch());
  onMount(() => dumps.watch());
  onMount(() => tweaks.watch());
  onMount(() => hwtests.listen());
  onMount(() => void hwtests.load());
  onMount(() => today.watch());

  // Stores what finished while the window was closed and restores a running capture, again when the watcher comes up.
  $effect(() => {
    void app.running;
    untrack(() => void captures.sync());
  });

  // The window behaves as an app: no page-wide select-all and no browser menu, except in text fields.
  onMount(() => {
    const editable = (target: EventTarget | null) =>
      target instanceof HTMLElement && (target.isContentEditable || target.matches("input, textarea"));
    const onkeydown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a" && !editable(event.target)) {
        event.preventDefault();
      }
    };
    const oncontextmenu = (event: MouseEvent) => {
      if (!editable(event.target)) event.preventDefault();
    };
    addEventListener("keydown", onkeydown);
    addEventListener("contextmenu", oncontextmenu);
    return () => {
      removeEventListener("keydown", onkeydown);
      removeEventListener("contextmenu", oncontextmenu);
    };
  });

  // Sets data-inactive on the root while another window is active; the title bar dims then.
  onMount(() => {
    const mark = () => {
      if (document.hasFocus()) delete document.documentElement.dataset.inactive;
      else document.documentElement.dataset.inactive = "";
    };
    mark();
    addEventListener("focus", mark);
    addEventListener("blur", mark);
    return () => {
      removeEventListener("focus", mark);
      removeEventListener("blur", mark);
    };
  });

  // A new screen starts at the top, with focus on the content so the keyboard continues there.
  $effect(() => {
    const key = router.key;
    if (key === shown) return;
    const first = shown === "";
    shown = key;
    if (first) return;
    void tick().then(() => {
      content?.scrollTo(0, 0);
      main?.focus({ preventScroll: true });
    });
  });
</script>

<div class="win">
  <div class="stage" bind:this={morph.stage}>
    {#if router.current.screen === "home" || morph.phase !== "idle"}
      <div class="layer home" bind:this={morph.home} data-morph-base inert={morph.phase !== "idle"}>
        {#if app.failure !== null}
          <div class="tb" data-tauri-drag-region></div>
          <main class="plain"><ErrorView error={app.failure} heading="h1" onretry={() => app.load()} /></main>
        {:else if app.loading}
          <div class="tb" data-tauri-drag-region></div>
          <main class="plain"><Loading /></main>
        {:else}
          <Home />
        {/if}
      </div>
    {/if}
    {#if router.current.screen !== "home"}
      <div class="layer page" bind:this={morph.page} inert={morph.phase !== "idle"}>
        <NavPane />
        <div class="ct">
          <div class="tb" data-tauri-drag-region></div>
          <div class="content" bind:this={content}>
            <div class="grid">
              <main id="main" tabindex="-1" bind:this={main}>
                {#if app.failure !== null}
                  <ErrorView error={app.failure} heading="h1" onretry={() => app.load()} />
                {:else if app.loading}
                  <Loading />
                {:else}
                  <Banner />
                  {#key router.key}
                    <div class="screen">
                      {#if router.current.screen === "occurrence"}
                        <Occurrence id={router.current.id} fromDay={router.current.fromDay} />
                      {:else if router.current.screen === "detail"}
                        <Detail kind={router.current.kind} parts={router.current.parts} />
                      {:else if router.current.screen === "disks"}
                        <Disks />
                      {:else if router.current.screen === "memory"}
                        <Memory />
                      {:else if router.current.screen === "events"}
                        <Events />
                      {:else if router.current.screen === "period"}
                        <PeriodPage start={router.current.start} fromDay={router.current.fromDay} />
                      {:else if router.current.screen === "programs"}
                        <Programs />
                      {:else if router.current.screen === "history"}
                        <History month={router.current.month} scope={router.current.scope} day={router.current.day} />
                      {:else if router.current.screen === "capture"}
                        <Capture id={router.current.id} />
                      {:else if router.current.screen === "captures"}
                        <Captures forOccurrence={router.current.forOccurrence} />
                      {:else if router.current.screen === "health"}
                        <Health area={router.current.area} />
                      {:else if router.current.screen === "dumps"}
                        <Dumps id={router.current.id} group={router.current.group} />
                      {:else if router.current.screen === "tweaks"}
                        <Tweaks key={router.current.key} group={router.current.group} />
                      {:else if router.current.screen === "tests"}
                        <Tests device={router.current.device} />
                      {:else if router.current.screen === "settings"}
                        <Settings />
                      {:else if router.current.screen === "activity"}
                        <Activity tab={router.current.tab} />
                      {/if}
                    </div>
                  {/key}
                {/if}
              </main>
              <Related />
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
  <WindowControls />
</div>
<Toast />
<UpdateDialog />

<style>
  .win {
    position: relative;
    height: 100vh;
  }
  .stage {
    position: absolute;
    inset: 0;
  }
  /* Each layer isolates its stacking, so a child's z-index never shows over the other layer. */
  .layer {
    position: absolute;
    inset: 0;
    isolation: isolate;
    background: var(--main);
  }
  .layer.home {
    overflow: hidden;
  }
  .layer.page {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
  }
  @media (min-width: 1280px) {
    .layer.page {
      grid-template-columns: 320px minmax(0, 1fr);
    }
  }
  :global(.morph) {
    position: absolute;
    z-index: 20;
    margin: 0;
    pointer-events: none;
    transform-origin: 0 0;
    will-change: transform, opacity;
  }
  /* Windows 10 draws the side and bottom edges of a frameless window but not the top one. */
  :global(:root[data-frame="win10"]:not([data-maximized])) .win {
    border-top: 1px solid var(--stroke);
  }
  /* The empty strip at the top of a column: it moves the window and leaves room for the caption buttons. */
  .tb {
    flex: none;
    height: 40px;
  }
  .plain {
    padding: 0 48px 48px;
  }
  .ct {
    display: grid;
    grid-template-rows: 40px minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
  }
  .content {
    min-height: 0;
    overflow: auto;
    scrollbar-gutter: stable;
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    gap: 56px;
    max-width: 1560px;
    padding: 0 28px 72px;
  }
  @media (max-width: 1499px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  @media (max-width: 1279px) {
    .grid {
      padding: 0 24px 64px;
    }
  }
  main {
    container: content / inline-size;
    min-width: 0;
    max-width: 1180px;
    outline: none;
  }
</style>
