<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { related } from "../lib/related.svelte";
</script>

<aside class="related" aria-label={t("ui.related.title")}>
  {#if related.links.length}
    <h2>{t("ui.related.title")}</h2>
    <ul>
      {#each related.links as link (link.href + link.label)}
        <li><a href={link.href}>{link.label}</a></li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .related {
    position: sticky;
    top: 0;
    align-self: start;
    padding-top: 62px;
  }
  @media (max-width: 1499px) {
    .related {
      display: none;
    }
  }
  h2 {
    margin: 0 0 14px;
    font: 600 14px/1.3 var(--f);
  }
  ul {
    display: grid;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  a {
    font-size: 14px;
    line-height: 1.4;
  }
</style>
