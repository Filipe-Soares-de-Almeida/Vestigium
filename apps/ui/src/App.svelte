<script lang="ts">
  import { onMount, tick, untrack } from "svelte";

  import Banner from "./components/Banner.svelte";
  import ErrorView from "./components/ErrorView.svelte";
  import Loading from "./components/Loading.svelte";
  import Mark from "./components/Mark.svelte";
  import NavPane from "./components/NavPane.svelte";
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
  import { t } from "./lib/i18n.svelte";
  import { router } from "./lib/router.svelte";
  import { updates } from "./lib/update.svelte";
  import Activity from "./screens/Activity.svelte";
  import Capture from "./screens/Capture.svelte";
  import Captures from "./screens/Captures.svelte";
  import Dumps from "./screens/Dumps.svelte";
  import Home from "./screens/Home.svelte";
  import Health from "./screens/Health.svelte";
  import Occurrence from "./screens/Occurrence.svelte";
  import Settings from "./screens/Settings.svelte";
  import Tests from "./screens/Tests.svelte";
  import Tweaks from "./screens/Tweaks.svelte";

  let content: HTMLElement;
  /** Screens whose page holds still while their panes scroll inside. */
  const STILL = new Set(["tests", "tweaks", "dumps", "home"]);

  let main: HTMLElement;
  let shown = "";

  onMount(() => app.start());
  onMount(() => captures.watch());
  onMount(() => void updates.start());
  onMount(() => void markFrame());
  onMount(() => health.watch());
  onMount(() => dumps.watch());
  onMount(() => tweaks.watch());
  onMount(() => hwtests.listen());

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
      content.scrollTo(0, 0);
      main.focus({ preventScroll: true });
    });
  });
</script>

<div class="win" class:home={router.current.screen === "home"}>
  <header class="titlebar" data-tauri-drag-region="deep">
    <Mark />
    <span>{t("ui.shell.title")}</span>
  </header>
  {#if router.current.screen !== "home"}<NavPane />{/if}
  <div class="content" bind:this={content}>
    <main id="main" tabindex="-1" bind:this={main} class:still={STILL.has(router.current.screen)}>
      {#if app.failure !== null}
        <ErrorView error={app.failure} heading="h1" onretry={() => app.load()} />
      {:else if app.loading}
        <Loading />
      {:else}
        <Banner />
        {#key router.key}
          <div class="screen">
            {#if router.current.screen === "home"}
              <Home />
            {:else if router.current.screen === "occurrence"}
              <Occurrence id={router.current.id} />
            {:else if router.current.screen === "capture"}
              <Capture id={router.current.id} />
            {:else if router.current.screen === "captures"}
              <Captures forOccurrence={router.current.forOccurrence} />
            {:else if router.current.screen === "health"}
              <Health area={router.current.area} />
            {:else if router.current.screen === "dumps"}
              <Dumps id={router.current.id} />
            {:else if router.current.screen === "tweaks"}
              <Tweaks key={router.current.key} />
            {:else if router.current.screen === "tests"}
              <Tests device={router.current.device} />
            {:else if router.current.screen === "settings"}
              <Settings />
            {:else}
              <Activity tab={router.current.tab} />
            {/if}
          </div>
        {/key}
      {/if}
    </main>
  </div>
  <WindowControls />
</div>
<Toast />
<UpdateDialog />

<style>
  .win {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
    grid-template-rows: 44px minmax(0, 1fr);
    height: 100vh;
  }
  @media (min-width: 1280px) {
    .win:not(.home) {
      grid-template-columns: 320px minmax(0, 1fr);
    }
  }
  .win.home {
    grid-template-columns: minmax(0, 1fr);
  }
  .win.home .content {
    border-left: 1px solid var(--stroke2);
    border-top-left-radius: 0;
  }
  .win.home main {
    max-width: none;
    padding: 0;
  }
  /* Windows 10 draws the side and bottom edges of a frameless window but not the top one. */
  :global(:root[data-frame="win10"]:not([data-maximized])) .win {
    border-top: 1px solid var(--stroke);
  }
  .titlebar {
    grid-row: 1;
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 44px;
    padding: 0 18px;
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
    user-select: none;
  }
  :global(:root[data-inactive]) .titlebar {
    color: var(--ink3);
  }
  .content {
    container: content / inline-size;
    min-height: 0;
    overflow: auto;
    background: var(--layer);
    border: 1px solid var(--stroke2);
    border-right: 0;
    border-bottom: 0;
    border-top-left-radius: 10px;
  }
  main {
    max-width: 1060px;
    padding: 26px 36px 70px;
    outline: none;
  }
  /* A screen that scrolls inside its own panel holds the page still. */
  .content:has(main.still) {
    overflow: clip;
  }
  main.still {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    height: 100%;
    padding-bottom: 20px;
  }
  main.still > :global(.screen) {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
  }
  @media (max-height: 520px) {
    .content:has(main.still) {
      overflow: auto;
    }
    main.still {
      display: block;
      height: auto;
    }
  }
</style>
