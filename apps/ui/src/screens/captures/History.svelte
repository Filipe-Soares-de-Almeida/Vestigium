<script lang="ts">
  import Block from "../../components/Block.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import type { Entry } from "../../lib/capture.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { compareLine, rowOf } from "../capture/words";

  let { entries, fresh }: { entries: Entry[]; fresh: number | null } = $props();

  const compare = $derived(compareLine(entries));
</script>

<Block title={t("ui.captures.history.title")}>
  {#if entries.length === 0}
    <p class="empty">{t("ui.captures.history.empty")}</p>
  {:else}
    {#if compare}
      <p class="compare"><span>{t("ui.captures.compare.label")}</span><b>{compare}</b></p>
    {/if}
    <RowList>
      {#each entries as entry (entry.summary.id)}
        {@const row = rowOf(entry)}
        <RowCard href={href.capture(row.id)} icon={row.icon} title={row.title} sub={row.sub}>
          {#snippet right()}
            {#if fresh === row.id}<span class="new">{t("ui.captures.history.new")}</span>{/if}
            <span class="when">{row.day}, {row.time}</span>
          {/snippet}
        </RowCard>
      {/each}
    </RowList>
  {/if}
</Block>

<style>
  .empty {
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .compare {
    display: grid;
    gap: 2px;
    margin: 0 0 12px;
    font-size: 14px;
  }
  .compare span {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .compare b {
    font-weight: 400;
  }
  .new {
    padding: 1px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--ink);
    font: 600 12px/1.6 var(--f);
  }
  .when {
    color: var(--ink2);
    font-size: 12.5px;
  }
</style>
