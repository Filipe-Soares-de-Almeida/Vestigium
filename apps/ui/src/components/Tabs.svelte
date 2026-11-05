<script lang="ts" generics="T extends string">
  interface Tab {
    id: T;
    label: string;
    /** Marks a tab whose content runs live. */
    live?: boolean;
  }

  let {
    label,
    tabs,
    current,
    onselect,
  }: {
    label: string;
    tabs: Tab[];
    current: T;
    onselect: (id: T) => void;
  } = $props();

  let buttons = $state<HTMLButtonElement[]>([]);

  // Left and Right, Home and End move between the tabs and open the one they reach.
  function onkeydown(event: KeyboardEvent): void {
    const at = tabs.findIndex((tab) => tab.id === current);
    const last = tabs.length - 1;
    const to =
      event.key === "ArrowRight"
        ? (at + 1) % tabs.length
        : event.key === "ArrowLeft"
          ? (at + last) % tabs.length
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : -1;
    if (to < 0) return;
    event.preventDefault();
    buttons[to]?.focus();
    if (tabs[to].id !== current) onselect(tabs[to].id);
  }
</script>

<div class="tabs" role="tablist" aria-label={label}>
  {#each tabs as tab, i (tab.id)}
    <button
      bind:this={buttons[i]}
      type="button"
      role="tab"
      id="tab-{tab.id}"
      aria-controls="tabpanel-{tab.id}"
      aria-selected={tab.id === current}
      tabindex={tab.id === current ? 0 : -1}
      onclick={() => tab.id !== current && onselect(tab.id)}
      {onkeydown}
    >
      {tab.label}{#if tab.live}<span class="live" aria-hidden="true"></span>{/if}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    margin: 12px 0 16px;
    border-bottom: 1px solid var(--stroke2);
  }
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    height: 38px;
    padding: 0 14px;
    border: 0;
    border-radius: 6px 6px 0 0;
    background: none;
    color: var(--ink2);
    font: 600 14px/1 var(--f);
    cursor: pointer;
  }
  button:hover {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 4%, transparent);
  }
  button[aria-selected="true"] {
    color: var(--ink);
  }
  button[aria-selected="true"]::after {
    content: "";
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: -1px;
    height: 3px;
    border-radius: 2px 2px 0 0;
    background: var(--accent);
  }
  .live {
    width: 7px;
    height: 7px;
    margin-left: 8px;
    border-radius: 50%;
    background: var(--ok);
  }
  @media (forced-colors: active) {
    button[aria-selected="true"]::after {
      forced-color-adjust: none;
      background: Highlight;
    }
    .live {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
