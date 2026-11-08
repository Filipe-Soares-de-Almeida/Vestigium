<script lang="ts">
  import { errorText } from "../lib/errors";
  import { t } from "../lib/i18n.svelte";

  let {
    error,
    onretry,
    heading = "h2",
  }: { error: unknown; onretry?: () => void; heading?: "h1" | "h2" } = $props();
</script>

<section class="failure">
  <svelte:element this={heading} class="title">{t("ui.common.error.title")}</svelte:element>
  <p>{errorText(error)}</p>
  {#if onretry}
    <button type="button" class="btn" onclick={onretry}>{t("ui.common.retry")}</button>
  {/if}
</section>

<style>
  .failure {
    max-width: 62ch;
  }
  .title {
    margin: 4px 0 0;
    font: 350 28px/1.2 var(--fd);
  }
  h2.title {
    font: 400 20px/1.3 var(--fd);
  }
  p {
    max-width: 72ch;
    margin: 10px 0 14px;
    font-size: 14px;
    line-height: 1.55;
  }
</style>
