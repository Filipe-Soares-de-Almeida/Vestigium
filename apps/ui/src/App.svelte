<script lang="ts">
  import { t } from "./i18n";
  import { agentStatus, timeline } from "./lib/api";

  const HOUR_MS = 3_600_000;
  const now = Date.now();
  const raw = Promise.all([agentStatus(), timeline(now - HOUR_MS, now, 120)]);
</script>

<main>
  <p>{t("ui.placeholder")}</p>
  {#await raw then [link, chart]}
    <pre>{JSON.stringify({ link, chart }, null, 2)}</pre>
  {:catch error}
    <pre>{JSON.stringify(error, null, 2)}</pre>
  {/await}
</main>
