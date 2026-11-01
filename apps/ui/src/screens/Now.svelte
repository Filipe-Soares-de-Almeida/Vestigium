<script lang="ts">
  import { liveStart } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import type { OccurrenceSummary } from "../lib/types";
  import { whileVisible } from "../lib/visibility";

  let latest = $state<OccurrenceSummary | null>(null);

  // The live stream runs only while this screen is shown, the window is visible and watching runs.
  $effect(() => {
    if (app.paused) return;
    return whileVisible(() =>
      liveStart((message) => {
        if (message.kind === "now") latest = message.frame.latest;
      }),
    );
  });
</script>

<h1>{t("ui.now.title")}</h1>
{#if !app.paused && latest}
  <p class="lede">
    <a href={href.occurrence(latest.id)}>{t("ui.now.last_occurrence")}</a>
  </p>
{/if}
