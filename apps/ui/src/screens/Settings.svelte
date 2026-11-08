<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { SETTINGS_PAGES } from "../lib/groups";
  import { router } from "../lib/router.svelte";
  import Appearance from "./settings/Appearance.svelte";
  import History from "./settings/History.svelte";
  import Language from "./settings/Language.svelte";
  import Privacy from "./settings/Privacy.svelte";
  import Reports from "./settings/Reports.svelte";
  import Updates from "./settings/Updates.svelte";
  import Watching from "./settings/Watching.svelte";

  type Category = (typeof SETTINGS_PAGES)[number];
  const current = $derived.by((): Category => {
    const route = router.current;
    const wanted = route.screen === "settings" ? route.category : null;
    return (SETTINGS_PAGES as readonly string[]).includes(wanted ?? "") ? (wanted as Category) : "appearance";
  });
  const settings = $derived(app.settings?.settings ?? null);
</script>

<h1>{t(`ui.settings.category.${current}`)}</h1>
{#if settings === null}
  <p class="lede">{t("ui.settings.unavailable")}</p>
{:else}
  <p class="lede">{app.editable ? t("ui.settings.lede") : t("ui.settings.read_only")}</p>
  <div class="setlist">
    {#if current === "appearance"}
      <Appearance {settings} />
    {:else if current === "reports"}
      <Reports {settings} />
    {:else if current === "watching"}
      <Watching />
    {:else if current === "history"}
      <History {settings} />
    {:else if current === "language"}
      <Language {settings} />
    {:else if current === "updates"}
      <Updates />
    {:else}
      <Privacy />
    {/if}
  </div>
{/if}

<style>
  .lede {
    max-width: 70ch;
    margin: 6px 0 18px;
    color: var(--ink2);
    font-size: 16px;
  }
  .setlist {
    display: grid;
    gap: 4px;
  }
  .setlist :global(select) {
    height: 32px;
    min-width: 0;
    padding: 0 10px;
    border: 1px solid color-mix(in srgb, var(--ink3) 55%, transparent);
    border-radius: 6px;
    background: var(--layer);
    box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--ink3) 55%, transparent);
    color: var(--ink);
    font: 400 14px/1.4 var(--f);
    accent-color: var(--accent);
    cursor: pointer;
  }
  .setlist :global(select:hover:enabled) {
    border-color: var(--ink3);
  }
  .setlist :global(select:focus) {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    outline: none;
  }
  .setlist :global(select:disabled) {
    color: var(--ink3);
    cursor: default;
  }
</style>
