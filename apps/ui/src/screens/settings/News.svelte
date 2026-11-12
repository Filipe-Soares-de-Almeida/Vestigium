<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../../components/BackLink.svelte";
  import Loading from "../../components/Loading.svelte";
  import PageHead from "../../components/PageHead.svelte";
  import VersionCard from "../../components/VersionCard.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { tagOf, wallOf } from "../../lib/notes";
  import { href } from "../../lib/router.svelte";
  import { type Release, updates } from "../../lib/update.svelte";

  // Every version as a card, newest first, with the one on offer on top; the wall scrolls, the page holds still.
  let notes = $state.raw<Release[] | null>(null);

  onMount(() => {
    void updates.allNotes().then((found) => (notes = found));
  });

  const wall = $derived(notes === null ? null : wallOf(notes, updates.offered));
</script>

<div class="fill">
  <BackLink href={href.settings("updates")} label={t("ui.settings.category.updates")} />
  <PageHead title={t("ui.news.title")} sub={t("ui.news.sub")} />
  {#if wall === null}
    <Loading />
  {:else if wall.length}
    <div class="wall">
      <div class="cards">
        {#each wall as release (release.version)}
          <VersionCard {release} tag={tagOf(release.version, updates.version, updates.found?.version ?? null)} />
        {/each}
      </div>
    </div>
  {:else}
    <p class="empty">{t("ui.settings.updates.news_none")}</p>
  {/if}
</div>

<style>
  .fill {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }
  .fill > :global(.back) {
    align-self: flex-start;
  }
  .wall {
    flex: 1;
    min-height: 0;
    margin-top: 20px;
    padding-right: 4px;
    overflow: auto;
  }
  .cards {
    display: grid;
    grid-auto-rows: 1fr;
    gap: 14px;
  }
  @container content (min-width: 900px) {
    .cards {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .empty {
    margin-top: 24px;
  }
</style>
