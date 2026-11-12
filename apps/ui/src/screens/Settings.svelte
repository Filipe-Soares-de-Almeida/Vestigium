<script lang="ts">
  import PageHead from "../components/PageHead.svelte";
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { SETTINGS_PAGES } from "../lib/groups";
  import { router } from "../lib/router.svelte";
  import { shellStatus } from "../lib/status";
  import { updates } from "../lib/update.svelte";
  import Appearance from "./settings/Appearance.svelte";
  import History from "./settings/History.svelte";
  import Language from "./settings/Language.svelte";
  import News from "./settings/News.svelte";
  import NewsVersion from "./settings/NewsVersion.svelte";
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
  /** The release notes pages under Updates, which read no setting. */
  const news = $derived(router.current.screen === "settings" ? router.current.news : null);
  /** The line under each page's title: what the page is about, the watcher's state or the version. */
  const sub = $derived.by(() => {
    if (current === "watching") return shellStatus().text;
    if (current === "updates" || current === "privacy") return t("ui.settings.updates.version_hint", { version: updates.version });
    return t(`ui.settings.sub.${current}`);
  });
</script>

{#if news?.version}
  <NewsVersion version={news.version} entry={news.entry} />
{:else if news}
  <News />
{:else if settings === null}
  <PageHead title={t(`ui.settings.category.${current}`)} sub={t("ui.settings.unavailable")} />
{:else}
  <PageHead
    title={t(`ui.settings.category.${current}`)}
    sub={app.editable ? sub : t("ui.settings.read_only")}
  />
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
  .setlist > :global(.sec:first-child) {
    margin-top: 24px;
    padding-top: 0;
    border-top: 0;
  }
</style>
