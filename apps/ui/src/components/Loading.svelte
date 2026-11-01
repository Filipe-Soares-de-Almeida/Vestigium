<script lang="ts">
  import { t } from "../lib/i18n.svelte";
</script>

<p class="loading" role="status">{t("ui.common.loading")}</p>

<style>
  .loading {
    margin: 0;
    padding: 6px 0;
    color: var(--ink2);
  }
</style>
