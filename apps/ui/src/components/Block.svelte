<script lang="ts">
  import type { Snippet } from "svelte";

  import Icon, { type IconName } from "./Icon.svelte";
  import SilentNote from "./SilentNote.svelte";

  let {
    title,
    icon,
    first = false,
    tight = false,
    fill = false,
    dim = false,
    notice,
    actions,
    children,
  }: {
    title: string;
    /** A line icon before the title, in the accent colour. */
    icon?: IconName;
    /** The block right under the page header has no divider above it. */
    first?: boolean;
    /** Less space around the divider, for dense pages. */
    tight?: boolean;
    /** The block takes the height of its column; its content decides how to use it. */
    fill?: boolean;
    /** The content shows old values: it is faded and cannot be pointed at. */
    dim?: boolean;
    /** A line under the title saying why the values are old; it fades the content too. */
    notice?: string;
    actions?: Snippet;
    children: Snippet;
  } = $props();
</script>

<section class="block" class:first class:tight class:fill class:dim={dim || notice !== undefined}>
  <header>
    <h2>{#if icon}<span class="ic"><Icon name={icon} /></span>{/if}{title}</h2>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </header>
  {#if notice !== undefined}<SilentNote text={notice} />{/if}
  {@render children()}
</section>

<style>
  .block {
    min-width: 0;
    margin-top: 32px;
    padding-top: 28px;
    border-top: 1px solid var(--rule);
  }
  .block.first {
    margin-top: 24px;
    padding-top: 0;
    border-top: 0;
  }
  .block.tight {
    margin-top: 20px;
    padding-top: 20px;
  }
  .block.tight header {
    margin-bottom: 8px;
  }
  .block.fill {
    display: flex;
    flex-direction: column;
  }
  .block.dim > :global(:not(header, .silent-note)) {
    opacity: 0.45;
    filter: saturate(0.4);
    pointer-events: none;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px 20px;
    min-height: 30px;
    margin-bottom: 12px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0;
    font: 400 20px/1.3 var(--fd);
  }
  .ic {
    display: flex;
    color: var(--accent);
  }
  .ic :global(svg) {
    width: 22px;
    height: 22px;
  }
  .ic :global(svg *) {
    vector-effect: non-scaling-stroke;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 16px;
  }
</style>
