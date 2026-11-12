<script lang="ts">
  import Block from "../../components/Block.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import Icon from "../../components/Icon.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
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
      <p class="cmp"><Icon name="captures" /><span>{compare}</span></p>
    {/if}
    <RowList>
      {#each entries as entry (entry.summary.id)}
        {@const row = rowOf(entry)}
        <RowCard
          href={href.capture(row.id)}
          icon="captures"
          title={row.title}
          sub="{row.day.charAt(0).toLocaleUpperCase() + row.day.slice(1)}, {row.time} · {row.sub}"
        >
          {#snippet right()}
            {#if fresh === row.id}<span class="new">{t("ui.captures.history.new")}</span>{/if}
            <StatusMark kind={row.result.kind} word={row.result.word} reveal />
            <StatusMark kind={row.cover.kind} word={row.cover.word} reveal />
          {/snippet}
        </RowCard>
      {/each}
    </RowList>
  {/if}
</Block>

<style>
  .cmp {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 12px;
    font-size: 14px;
  }
  .cmp :global(svg) {
    color: var(--ink2);
  }
  .new {
    padding: 1px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--ink);
    font: 600 12px/1.6 var(--f);
  }
</style>
