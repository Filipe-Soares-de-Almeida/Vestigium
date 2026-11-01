<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
</script>

<h1>{t("ui.settings.title")}</h1>
{#if !app.editable}
  <p class="lede">{t("ui.settings.read_only")}</p>
{/if}
