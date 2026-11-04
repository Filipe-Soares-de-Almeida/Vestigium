<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { go } from "../lib/router.svelte";
  import Icon from "./Icon.svelte";

  /** The head of a second-level screen: an arrow back to the level above, then where you are. */
  let {
    parent,
    current,
    label,
  }: {
    parent: { href: string; name: string };
    current: string;
    /** What the navigation landmark is called. */
    label: string;
  } = $props();

  // Alt+← and the mouse's back button go up a level, as the arrow does.
  $effect(() => {
    const up = parent.href;
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

<nav class="levelhead" aria-label={label}>
  <a
    class="up"
    href={parent.href}
    aria-label={t("ui.common.back_to", { place: parent.name })}
    title={t("ui.common.back_to.tip", { place: parent.name })}
  >
    <Icon name="arrow_left" />
  </a>
  <span aria-current="page">{current}</span>
</nav>

<style>
  .levelhead {
    display: flex;
    align-items: center;
    gap: 12px;
    font: 600 28px/1.25 var(--fd);
    letter-spacing: -0.005em;
  }
  .up {
    display: grid;
    flex: none;
    place-items: center;
    width: 34px;
    height: 34px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--card);
    color: var(--ink);
  }
  .up:hover {
    background: var(--card2);
  }
  .up :global(svg) {
    width: 18px;
    height: 18px;
  }
  span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
