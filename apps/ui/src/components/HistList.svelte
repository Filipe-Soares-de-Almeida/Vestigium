<script lang="ts" module>
  import type { StatusKind } from "./StatusMark.svelte";

  /** One line of a dated history: when, what, how it ended, and where it opens. */
  export interface HistEntry {
    key: string;
    date: string;
    title: string;
    sub?: string;
    mark?: { kind: StatusKind; word?: string };
    href?: string | null;
  }
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import StatusMark from "./StatusMark.svelte";

  let { entries }: { entries: HistEntry[] } = $props();
</script>

<ol class="hist">
  {#each entries as entry (entry.key)}
    <li>
      {#if entry.href}
        <a class="row" href={entry.href} data-reveal>{@render body(entry)}<span class="ch"><Icon name="chevron" /></span></a>
      {:else}
        <span class="row plain">{@render body(entry)}</span>
      {/if}
    </li>
  {/each}
</ol>

{#snippet body(entry: HistEntry)}
  <span class="d">{entry.date}</span>
  <span class="m">
    <span>{entry.title}</span>
    {#if entry.sub}<small>{entry.sub}</small>{/if}
  </span>
  {#if entry.mark}<StatusMark kind={entry.mark.kind} word={entry.mark.word} reveal />{/if}
{/snippet}

<style>
  .hist {
    margin: 0;
    padding: 0;
    font-size: 14px;
    list-style: none;
  }
  .hist li + li {
    border-top: 1px solid var(--stroke2);
  }
  .row {
    display: grid;
    grid-template-columns: 128px minmax(0, 1fr) auto 16px;
    gap: 16px;
    align-items: center;
    margin: 0 -8px;
    padding: 9px 8px;
    border-radius: 6px;
    color: var(--ink);
    text-decoration: none;
  }
  a.row:hover {
    background: var(--row-hover);
    text-decoration: none;
  }
  a.row:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  .row.plain {
    grid-template-columns: 128px minmax(0, 1fr) auto;
  }
  .d {
    color: var(--ink2);
    font-size: 13px;
  }
  .m {
    display: grid;
    min-width: 0;
  }
  small {
    color: var(--ink2);
    font-size: 13px;
  }
  .ch {
    display: grid;
    grid-column: -2;
    color: var(--ink3);
  }
</style>
