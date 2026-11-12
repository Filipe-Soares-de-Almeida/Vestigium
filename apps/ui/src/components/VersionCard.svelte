<script lang="ts">
  import { formatNumber } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { entryKey, heading, highlights, releaseDay, type Tag } from "../lib/notes";
  import { href } from "../lib/router.svelte";
  import type { Release } from "../lib/update.svelte";

  // A version at a glance: its summary, its first highlights and a count per group, which open its page.
  let {
    release,
    tag = null,
    max = 5,
    bare = false,
    onnavigate,
  }: {
    release: Release;
    tag?: Tag;
    /** How many highlights to list. */
    max?: number;
    /** Without its header and its frame, inside a dialog. */
    bare?: boolean;
    /** Called as one of its links opens a page. */
    onnavigate?: () => void;
  } = $props();

  const id = $derived(`version-${release.version.replace(/[^\w-]/g, "-")}`);
  const top = $derived(highlights(release, max));
  const day = $derived(releaseDay(release.date));
</script>

<article class="vc" class:bare aria-labelledby={bare ? undefined : id}>
  {#if !bare}
    <header class="vh">
      <h2 {id}>{t("ui.update.version", { version: release.version })}</h2>
      {#if tag}<span class="tag" class:av={tag === "available"}>{t(`ui.news.tag.${tag}`)}</span>{/if}
      {#if day}<span class="day">{day}</span>{/if}
    </header>
  {/if}
  {#if release.summary}<p class="sum">{release.summary}</p>{/if}
  {#if top.length}
    <div class="hl">
      <h3>{t("ui.news.highlights")}</h3>
      <ul>
        {#each top as entry (entryKey(entry))}
          <li>
            <a href={href.news(release.version, entryKey(entry))} title={t("ui.news.open_entry")} onclick={onnavigate}>
              <span class="clamp">{entry.text}</span>
            </a>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
  <footer class="vf">
    <ul class="cnts" aria-label={t("ui.news.groups")}>
      {#each release.groups as group (group.kind)}
        <li class="cnt">{heading(group.kind)} <b>{formatNumber(group.items.length)}</b></li>
      {/each}
    </ul>
    <a class={bare ? "all" : "btn"} href={href.news(release.version)} onclick={onnavigate}>{t("ui.news.see_all")}</a>
  </footer>
</article>

<style>
  .vc {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    padding: 18px 20px 16px;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--card);
  }
  .vc.bare {
    gap: 10px;
    padding: 0;
    border: 0;
    background: none;
  }
  .vh {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
  }
  h2 {
    margin: 0;
    font: 600 17px/1.3 var(--fd);
  }
  .tag {
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--ink2);
    font: 400 12px/1.5 var(--f);
    white-space: nowrap;
  }
  .tag.av {
    background: var(--accent-t);
    color: var(--ink);
  }
  .day {
    margin-left: auto;
    color: var(--ink2);
    font-size: 13px;
  }
  .sum {
    max-width: 76ch;
    margin: 0;
    color: var(--ink);
    font-size: 14.5px;
    line-height: 1.55;
  }
  h3 {
    margin: 0 0 6px;
    color: var(--ink2);
    font: 600 12.5px/1.3 var(--f);
  }
  .hl ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .hl li {
    position: relative;
    padding-left: 16px;
    font-size: 14px;
    line-height: 1.45;
  }
  .hl li::before {
    content: "";
    position: absolute;
    top: 0.6em;
    left: 3px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--ink3);
  }
  .hl a {
    display: block;
    border-radius: 4px;
    color: var(--ink);
    text-decoration: none;
  }
  .clamp {
    display: -webkit-box;
    overflow: hidden;
    font-weight: 600;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  .hl a:hover .clamp {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .vf {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid var(--stroke2);
  }
  .bare .vf {
    margin-top: 2px;
  }
  .cnts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .cnt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--stroke);
    border-radius: 999px;
    color: var(--ink2);
    font: 400 12.5px/1 var(--f);
    white-space: nowrap;
  }
  .cnt b {
    color: var(--ink);
    font-weight: 600;
  }
  .all {
    font-size: 14px;
    line-height: 1.5;
  }
  @media (forced-colors: active) {
    .tag,
    .cnt {
      border: 1px solid CanvasText;
    }
  }
</style>
