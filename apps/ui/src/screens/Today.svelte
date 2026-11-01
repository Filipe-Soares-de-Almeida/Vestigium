<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
</script>

<h1>{t("ui.today.title")}</h1>
{#if app.history === "empty"}
  <p class="lede">{t("ui.today.first_use")}</p>
  <a class="btn accent" href={href.now()}>{t("ui.today.see_now")}</a>
{/if}
