<script lang="ts">
  import { go } from "../lib/router.svelte";
  import Icon from "./Icon.svelte";

  /** Top of a sub-screen: leads back to the list it was opened from. */
  let { href, label }: { href: string; label: string } = $props();

  // Alt+← and the mouse's back button go up a level, as the link does.
  $effect(() => {
    const up = href;
    const key = (event: KeyboardEvent): void => {
      if (event.altKey && event.key === "ArrowLeft") {
        event.preventDefault();
        go(up);
      }
    };
    const stop = (event: MouseEvent): void => {
      if (event.button === 3) event.preventDefault();
    };
    const mouse = (event: MouseEvent): void => {
      if (event.button !== 3) return;
      event.preventDefault();
      go(up);
    };
    window.addEventListener("keydown", key);
    window.addEventListener("mousedown", stop);
    window.addEventListener("mouseup", mouse);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("mousedown", stop);
      window.removeEventListener("mouseup", mouse);
    };
  });
</script>

<a class="back" {href}>
  <Icon name="arrow_left" />
  <span>{label}</span>
</a>

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    margin: 0 0 10px -10px;
    padding: 0 14px 0 10px;
    border-radius: 6px;
    color: var(--ink);
    font-size: 13.5px;
    text-decoration: none;
  }
  .back :global(svg) {
    color: var(--ink2);
  }
  .back:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .back:active {
    background: var(--row-press);
  }
  .back:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: 1px;
  }
</style>
