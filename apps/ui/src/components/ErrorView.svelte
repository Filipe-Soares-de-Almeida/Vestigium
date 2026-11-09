<script lang="ts">
  import { errorText } from "../lib/errors";
  import { t } from "../lib/i18n.svelte";

  let {
    error,
    onretry,
    heading = "h2",
  }: { error: unknown; onretry?: () => void; heading?: "h1" | "h2" } = $props();

  /** What the failing side reported, for a bug report; empty when it said nothing. */
  const detail = $derived.by(() => {
    const message = (error as { message?: unknown } | null)?.message;
    if (typeof message === "string") return message;
    return error instanceof Error ? error.message : typeof error === "string" ? error : "";
  });
</script>

<section class="failure">
  <svelte:element this={heading} class="title">{t("ui.common.error.title")}</svelte:element>
  <p>{errorText(error)}</p>
  {#if onretry}
    <button type="button" class="btn" onclick={onretry}>{t("ui.common.retry")}</button>
  {/if}
  {#if detail}
    <details>
      <summary>{t("ui.common.error.details")}</summary>
      <code>{detail}</code>
    </details>
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
  details {
    margin-top: 18px;
    color: var(--ink2);
    font-size: 13px;
  }
  summary {
    cursor: pointer;
  }
  code {
    display: block;
    margin-top: 8px;
    color: var(--ink2);
    font: 12.5px/1.5 var(--mono);
    overflow-wrap: anywhere;
    user-select: text;
  }
  p {
    max-width: 72ch;
    margin: 10px 0 14px;
    font-size: 14px;
    line-height: 1.55;
  }
</style>
