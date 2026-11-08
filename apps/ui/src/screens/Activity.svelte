<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import type { ActivityTab } from "../lib/router.svelte";
  import Now from "./Now.svelte";
  import Today from "./Today.svelte";

  let { tab }: { tab: ActivityTab } = $props();
</script>

{#if tab === "now"}
  <h1>{t("ui.activity.tab.now")}</h1>
  <Now />
{:else}
  <Today />
{/if}
