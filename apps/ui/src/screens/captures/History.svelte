<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import type { Entry } from "../../lib/capture.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { compareLine, rowOf } from "../capture/words";

  let { entries, fresh }: { entries: Entry[]; fresh: number | null } = $props();

  const compare = $derived(compareLine(entries));
</script>

<section class="card list" aria-labelledby="history-title">
  <h2 id="history-title">{t("ui.captures.history.title")}</h2>
  {#if entries.length === 0}
    <p class="empty">{t("ui.captures.history.empty")}</p>
  {:else}
    {#if compare}
      <p class="compare"><span>{t("ui.captures.compare.label")}</span><b>{compare}</b></p>
    {/if}
    <ul>
      {#each entries as entry (entry.summary.id)}
        {@const row = rowOf(entry)}
        <li>
          <a class="row" href={href.capture(row.id)}>
            <span class="icochip" class:quiet={row.quiet}><Icon name={row.icon} /></span>
            <span class="what">
              <b>{row.title}{#if fresh === row.id}<span class="pill">{t("ui.captures.history.new")}</span>{/if}</b>
              <small>{row.sub}</small>
            </span>
            <span class="when">{row.day}<br />{row.time}</span>
            <span class="chev"><Icon name="chevron" /></span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .list {
    padding: 10px;
  }
  h2 {
    margin: 0;
    padding: 6px 10px 2px;
    font-size: 15px;
    font-family: var(--f);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .empty {
    margin: 0;
    padding: 6px 10px 8px;
    color: var(--ink2);
    font-size: 13.5px;
  }
  .compare {
    display: grid;
    gap: 2px;
    margin: 6px 10px 8px;
    padding: 8px 12px;
    border: 1px solid var(--stroke2);
    border-radius: 10px;
    background: var(--card2);
  }
  .compare span {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .compare b {
    font-weight: 600;
  }
  .row {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto 14px;
    gap: 12px;
    align-items: center;
    padding: 9px 10px;
    border-radius: 8px;
    color: inherit;
    text-decoration: none;
  }
  .row:hover {
    background: var(--card2);
  }
  .icochip {
    background: var(--busy-t);
    color: var(--busy);
  }
  .icochip.quiet {
    background: var(--card2);
    border: 1px solid var(--stroke2);
    color: var(--ink2);
  }
  .what b {
    display: block;
    font-weight: 600;
    line-height: 1.3;
  }
  .what b .pill {
    margin-left: 8px;
    vertical-align: 1px;
  }
  small {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .when {
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.35;
    text-align: right;
    white-space: nowrap;
  }
  .chev {
    display: grid;
    color: var(--ink3);
  }
</style>
