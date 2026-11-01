<script lang="ts">
  import Icon from "../components/Icon.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";

  let { id }: { id: number } = $props();
</script>

<nav class="crumb" aria-label={t("ui.occurrence.path")}>
  <a href={href.today()}>{t("ui.shell.nav.today")}</a>
  <Icon name="chevron" />
  <h1 aria-current="page">{t("ui.occurrence.title", { id: String(id) })}</h1>
</nav>

<style>
  .crumb {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font: 600 28px/1.25 var(--fd);
  }
  .crumb h1 {
    font: inherit;
  }
  .crumb a {
    color: var(--ink2);
    text-decoration: none;
  }
  .crumb a:hover {
    color: var(--ink);
  }
  .crumb :global(svg) {
    color: var(--ink3);
  }
</style>
