<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { DetailLevel } from "../../lib/types";
  import { type Item, keyOfItem } from "./day";
  import EpisodeRow from "./EpisodeRow.svelte";
  import OccurrenceRow from "./OccurrenceRow.svelte";

  let { items, level }: { items: Item[]; level: DetailLevel } = $props();
</script>

<Block title={t("ui.today.moments.title")}>
  {#if items.length === 0}
    <p class="empty">{t("ui.today.moments.none")}</p>
  {:else}
    <ul>
      {#each items as item, i (keyOfItem(item, i))}
        <li>
          {#if item.kind === "occurrence"}
            <OccurrenceRow o={item.occurrence} {level} />
          {:else}
            <EpisodeRow e={item.episode} {level} byTest={item.byTest} />
          {/if}
        </li>
      {/each}
    </ul>
    <p class="more"><a href={href.events()}>{t("ui.today.moments.all")}</a></p>
  {/if}
</Block>

<style>
  ul {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
