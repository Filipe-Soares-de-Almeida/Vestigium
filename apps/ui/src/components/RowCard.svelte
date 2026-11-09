<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon, { type IconName } from "./Icon.svelte";

  let {
    href,
    onclick,
    icon,
    title,
    sub,
    right,
    chevron = true,
    static: still = false,
  }: {
    /** A link row; without it the row is a button. */
    href?: string;
    onclick?: () => void;
    icon?: IconName;
    title: string;
    sub?: string;
    /** Trailing content, usually a StatusMark with `reveal`. */
    right?: Snippet;
    chevron?: boolean;
    /** A row that only shows a value and opens nothing. */
    static?: boolean;
  } = $props();
</script>

<li>
  {#if still}
    <div class="card still">
      {@render body()}
    </div>
  {:else if href}
    <a class="card" {href} data-reveal>
      {@render body()}
    </a>
  {:else}
    <button class="card" type="button" {onclick} data-reveal>
      {@render body()}
    </button>
  {/if}
</li>

{#snippet body()}
  {#if icon}<span class="icon"><Icon name={icon} /></span>{/if}
  <span class="main">
    <span class="title">{title}</span>
    {#if sub}<span class="sub">{sub}</span>{/if}
  </span>
  {#if right}<span class="right">{@render right()}</span>{/if}
  {#if chevron}<span class="chev"><Icon name="chevron" /></span>{/if}
{/snippet}

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 16px;
    width: 100%;
    min-height: 56px;
    padding: 10px 12px;
    border: 0;
    border-radius: 6px;
    background: var(--row);
    color: var(--ink);
    font: inherit;
    text-align: left;
    text-decoration: none;
    cursor: pointer;
  }
  .card:hover {
    background: var(--row-hover);
    text-decoration: none;
  }
  .card.still,
  .card.still:hover {
    background: var(--row);
    cursor: default;
  }
  .card:active {
    background: var(--row-press);
  }
  .card:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  .icon {
    display: flex;
    flex: none;
    color: var(--ink2);
  }
  .icon :global(svg) {
    width: 24px;
    height: 24px;
  }
  .main {
    display: grid;
    flex: 1;
    gap: 1px;
    min-width: 0;
  }
  .title {
    overflow: hidden;
    font-size: 14.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub {
    overflow: hidden;
    color: var(--ink2);
    font-size: 13px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .right {
    display: flex;
    flex: none;
    align-items: center;
    gap: 16px;
    color: var(--ink2);
    font-size: 13px;
    white-space: nowrap;
  }
  .chev {
    display: flex;
    flex: none;
    color: var(--ink3);
  }
  @media (forced-colors: active) {
    .card:focus-visible {
      outline-color: Highlight;
    }
  }
</style>
