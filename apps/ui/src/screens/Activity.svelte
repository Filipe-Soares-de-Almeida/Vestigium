<script lang="ts">
  import Tabs from "../components/Tabs.svelte";
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { type ActivityTab, href } from "../lib/router.svelte";
  import Now from "./Now.svelte";
  import Today from "./Today.svelte";

  let { tab }: { tab: ActivityTab } = $props();

  const tabs = $derived([
    { id: "today" as const, label: t("ui.activity.tab.today") },
    { id: "now" as const, label: t("ui.activity.tab.now"), live: app.running && !app.paused },
  ]);

  /** Switching tabs replaces the address, so Back leaves the screen instead of walking the tabs. */
  function select(next: ActivityTab): void {
    location.replace(href.activity(next));
  }
</script>

<h1>{t("ui.activity.title")}</h1>
<Tabs label={t("ui.activity.title")} {tabs} current={tab} onselect={select} />
<div role="tabpanel" id="tabpanel-{tab}" aria-labelledby="tab-{tab}">
  {#if tab === "now"}
    <Now />
  {:else}
    <Today />
  {/if}
</div>
