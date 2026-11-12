<script lang="ts">
  import { aboutDialog } from "../lib/aboutDialog.svelte";
  import { buildGroups, followed, groupOf, itemHref } from "../lib/groups";
  import { t } from "../lib/i18n.svelte";
  import { href, router } from "../lib/router.svelte";
  import { shellStatus } from "../lib/status";
  import { updates } from "../lib/update.svelte";
  import Finder from "./Finder.svelte";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";
  import StatusMark from "./StatusMark.svelte";
  import ToneMark from "./ToneMark.svelte";

  const status = $derived(shellStatus());
  const group = $derived.by(() => {
    const groups = buildGroups();
    return { groups, current: groupOf(groups, router.current, router.group) };
  });
</script>

<nav class="pane" aria-label={t("ui.shell.nav.label")}>
  <div class="tb" data-tauri-drag-region="deep">
    <button
      type="button"
      class="name"
      aria-haspopup="dialog"
      aria-label={t("ui.about.title")}
      title={t("ui.about.title")}
      onclick={() => aboutDialog.show()}
    >
      <Mark size={16} />
      <span>{t("ui.shell.title")}</span>
    </button>
  </div>
  <a class="home" href={href.home()}>
    <Icon name="home" />
    <span>{t("ui.nav.home")}</span>
  </a>
  <div class="find"><Finder id="side-find" /></div>
  {#if group.current}
    {@const current = group.current}
    <h2 class="group" id="nav-group">{current.name}</h2>
    <ul class="items" aria-labelledby="nav-group">
      {#each current.items as item (item.href)}
        <li>
          <a
            class="item"
            data-reveal
            href={itemHref(current, item)}
            aria-current={item.match(followed(router.current)) ? "page" : undefined}
          >
            <Icon name={item.icon} />
            <span class="label">{item.label}</span>
            {#if item.status}<StatusMark kind={item.status} word={item.statusWord} size={14} reveal />{/if}
            {#if item.meta}<span class="meta">{item.meta}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="foot">
    <p class="state"><ToneMark tone={status.tone} />{status.text}</p>
    {#if updates.version}
      <button type="button" class="ver" aria-haspopup="dialog" onclick={() => aboutDialog.show()}>
        <Icon name="info" />{t("ui.settings.updates.version_hint", { version: updates.version })}
      </button>
    {/if}
  </div>
</nav>

<style>
  .pane {
    position: relative;
    z-index: 2;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--band);
  }
  .tb {
    display: flex;
    flex: none;
    align-items: center;
    height: 40px;
    padding-left: 8px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 10px 0 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--ink2);
    font-size: 12px;
    cursor: pointer;
  }
  .name:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .name:active {
    background: var(--row-press);
  }
  :global(:root[data-inactive]) .name {
    color: var(--ink3);
  }
  .home {
    display: flex;
    flex: none;
    align-items: center;
    gap: 16px;
    height: 40px;
    margin: 8px 8px 0;
    padding: 0 12px;
    border-radius: 6px;
    color: var(--ink);
    font-size: 14px;
    text-decoration: none;
  }
  .home:hover,
  .item:hover {
    background: var(--hover);
    text-decoration: none;
  }
  .home > :global(svg),
  .item > :global(svg:first-child) {
    flex: none;
    width: 16px;
    height: 16px;
    color: var(--ink2);
  }
  .find {
    flex: none;
    padding: 10px 16px 0;
  }
  .group {
    flex: none;
    margin: 20px 0 10px;
    padding: 20px 20px 0;
    border-top: 1px solid var(--rule);
    font: 600 15px/1.3 var(--f);
  }
  .items {
    display: grid;
    flex: 1;
    align-content: start;
    gap: 2px;
    min-height: 0;
    margin: 0;
    padding: 0 2px 12px 8px;
    overflow: auto;
    list-style: none;
    scrollbar-gutter: stable;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    height: 40px;
    padding: 0 10px 0 12px;
    border-radius: 6px;
    color: var(--ink);
    font-size: 14px;
    text-decoration: none;
  }
  .item:active {
    background: var(--row-press);
  }
  .item[aria-current="page"] {
    background: var(--hover);
  }
  .item[aria-current="page"] :global(svg:first-child) {
    color: var(--ink);
  }
  .item[aria-current="page"]::before {
    content: "";
    position: absolute;
    top: 50%;
    left: 0;
    width: 3px;
    height: 16px;
    margin-top: -8px;
    border-radius: 2px;
    background: var(--accent);
  }
  .item:focus-visible,
  .home:focus-visible,
  .name:focus-visible,
  .ver:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    flex: none;
    color: var(--ink3);
    font-size: 12.5px;
  }
  .foot {
    display: grid;
    flex: none;
    gap: 2px;
    padding: 9px 12px 10px;
    border-top: 1px solid var(--stroke2);
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
  }
  .state {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    margin: 0;
    padding: 0 8px;
  }
  .ver {
    display: flex;
    align-items: center;
    gap: 8px;
    width: max-content;
    max-width: 100%;
    height: 28px;
    padding: 0 10px 0 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--ink2);
    font: inherit;
    cursor: pointer;
  }
  .ver:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .ver:active {
    background: var(--row-press);
  }
  .ver > :global(svg) {
    flex: none;
    width: 14px;
    height: 14px;
  }
  @media (forced-colors: active) {
    .item[aria-current="page"]::before {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
