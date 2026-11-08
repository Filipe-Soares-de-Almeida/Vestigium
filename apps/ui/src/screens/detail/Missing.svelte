<script lang="ts">
  import { t } from "../../lib/i18n.svelte";

  /** What a detail page says when its item is gone or was never kept. */
  let { body }: { body?: string } = $props();
</script>

<p class="missing"><b>{t("ui.detail.missing.title")}</b> {body ?? t("ui.detail.missing.body")}</p>

<style>
  .missing {
    max-width: 70ch;
    margin: 24px 0 0;
    color: var(--ink2);
    font-size: 15px;
    line-height: 1.5;
  }
  b {
    color: var(--ink);
    font-weight: 600;
  }
</style>
