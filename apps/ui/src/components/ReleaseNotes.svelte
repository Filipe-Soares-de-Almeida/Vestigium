<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import type { NotesGroup, Release } from "../lib/update.svelte";

  // A version's notes: its headings in the window's words, and the entries under each.
  let { releases, groups }: { releases?: Release[]; groups?: NotesGroup[] } = $props();

  const KINDS = ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"];
  const heading = (kind: string) => (KINDS.includes(kind) ? t(`ui.update.group.${kind}`) : kind);
</script>

{#snippet list(items: NotesGroup[])}
  {#each items as group (group.kind)}
    <h4>{heading(group.kind)}</h4>
    <ul>
      {#each group.items as item, i (i)}<li>{item}</li>{/each}
    </ul>
  {/each}
{/snippet}

<div class="notes">
  {#if groups}
    {@render list(groups)}
  {/if}
  {#each releases ?? [] as release (release.version)}
    <section>
      <p class="ver"><b>{t("ui.update.version", { version: release.version })}</b>{#if release.date}<small>{release.date}</small>{/if}</p>
      {@render list(release.groups)}
    </section>
  {/each}
</div>

<style>
  .notes {
    display: grid;
    gap: 14px;
  }
  .ver {
    display: flex;
    gap: 10px;
    align-items: baseline;
    margin: 0;
  }
  .ver b {
    font: 600 15px/1.3 var(--fd);
  }
  .ver small {
    color: var(--ink3);
    font-size: 12.5px;
  }
  h4 {
    margin: 8px 0 4px;
    color: var(--ink2);
    font: 600 12.5px/1.35 var(--f);
  }
  ul {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 0 0 0 18px;
    font-size: 14px;
    line-height: 1.45;
  }
  li::marker {
    color: var(--ink3);
  }
</style>
