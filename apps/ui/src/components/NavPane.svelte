<script lang="ts">
  import { buildGroups, groupOf, itemHref } from "../lib/groups";
  import { t } from "../lib/i18n.svelte";
  import { href, router } from "../lib/router.svelte";
  import { shellStatus } from "../lib/status";
  import Finder from "./Finder.svelte";
  import Icon from "./Icon.svelte";
  import StatusMark from "./StatusMark.svelte";

  const status = $derived(shellStatus());
  const group = $derived.by(() => {
    const groups = buildGroups();
    return { groups, current: groupOf(groups, router.current, router.group) };
  });
</script>

<nav class="pane" aria-label={t("ui.shell.nav.label")}>
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
            aria-current={item.match(router.current) ? "page" : undefined}
          >
            <Icon name={item.icon} />
            <span class="label">{item.label}</span>
            {#if item.status}<StatusMark kind={item.status} size={14} reveal />{/if}
            {#if item.meta}<span class="meta">{item.meta}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>
  {/if}
  <p class="foot"><i class={status.tone} aria-hidden="true"></i>{t(status.key)}</p>
</nav>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 4px 0 0;
  }
  .home {
    display: flex;
    flex: none;
    align-items: center;
    gap: 16px;
    height: 40px;
    margin: 0 8px;
    padding: 0 12px;
    border-radius: 6px;
    color: var(--ink);
    font-size: 14px;
    text-decoration: none;
  }
  .home:hover,
  .item:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .home :global(svg),
  .item :global(svg) {
    flex: none;
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
    background: color-mix(in srgb, var(--ink) 6%, transparent);
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
  .home:focus-visible {
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
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 12px 20px 14px;
    border-top: 1px solid var(--stroke2);
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
  }
  .foot i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .foot i.ok {
    background: var(--good);
  }
  .foot i.paused {
    background: var(--press);
  }
  .foot i.off {
    border: 1.5px solid var(--ink3);
  }
  @media (forced-colors: active) {
    .foot i,
    .foot i.ok,
    .foot i.paused {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .foot i.off {
      background: Canvas;
      border-color: CanvasText;
    }
    .item[aria-current="page"]::before {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
