<script lang="ts">
  import { errorText } from "../lib/errors";
  import { t } from "../lib/i18n.svelte";

  let {
    error,
    onretry,
    heading = "h2",
  }: { error: unknown; onretry?: () => void; heading?: "h1" | "h2" } = $props();
</script>

<section class="card failure">
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
    font: 600 18px/1.3 var(--fd);
    margin: 0 0 6px;
  }
  p {
    margin: 0 0 14px;
    color: var(--ink2);
  }
</style>
