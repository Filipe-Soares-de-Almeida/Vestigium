<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon, { type IconName } from "./Icon.svelte";

  let {
    href,
    onclick,
    icon,
    lead,
    title,
    sub,
    right,
    chevron = true,
    static: still = false,
    open = $bindable(false),
    expand,
  }: {
    /** A link row; without it the row is a button. */
    href?: string;
    onclick?: () => void;
    icon?: IconName;
    /** Leading content in place of the icon, such as a status symbol. */
    lead?: Snippet;
    title: string;
    sub?: string;
    /** Trailing content, usually a StatusMark with `reveal`. */
    right?: Snippet;
    chevron?: boolean;
    /** A row that only shows a value and opens nothing. */
    static?: boolean;
    /** What the row opens below itself; with it the row is a button that shows or hides this. */
    expand?: Snippet;
    open?: boolean;
  } = $props();

  const id = $props.id();
</script>

<li class:expandable={expand !== undefined} class:open>
  {#if expand}
    <button class="card" type="button" aria-expanded={open} aria-controls={open ? `${id}-body` : undefined} onclick={() => (open = !open)} data-reveal>
      {@render body()}
    </button>
    {#if open}<div class="expand" id="{id}-body">{@render expand()}</div>{/if}
  {:else if still}
    <div class="card still" data-reveal>
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
  {#if lead}<span class="icon">{@render lead()}</span>{:else if icon}<span class="icon"><Icon name={icon} /></span>{/if}
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
    transition: transform 0.12s;
  }
  .expandable .chev {
    transform: rotate(90deg);
  }
  .expandable.open .chev {
    transform: rotate(-90deg);
  }
  .expandable.open > .card {
    border-radius: 6px 6px 0 0;
  }
  .expand {
    padding: 14px 16px 18px 52px;
    border-top: 1px solid var(--stroke2);
    border-radius: 0 0 6px 6px;
    background: var(--row);
  }
  .expand > :global(:first-child) {
    margin-top: 0;
  }
  .expand > :global(:last-child) {
    margin-bottom: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .chev {
      transition: none;
    }
  }
  @media (forced-colors: active) {
    .card:focus-visible {
      outline-color: Highlight;
    }
  }
</style>
