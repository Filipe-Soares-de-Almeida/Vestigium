<script lang="ts">
  import { buildGroups } from "../lib/groups";
  import { t } from "../lib/i18n.svelte";
  import { go } from "../lib/router.svelte";
  import { buildIndex, highlight, search, type SearchEntry } from "../lib/search";
  import Icon from "./Icon.svelte";

  let { id, big = false, autofocus = false }: { id: string; big?: boolean; autofocus?: boolean } = $props();

  let query = $state("");
  let active = $state(0);
  let opened = $state(false);
  let input: HTMLInputElement;

  const index = $derived(buildIndex(buildGroups()));
  const found = $derived(search(index, query.trim()));
  const options = $derived(found.groups.flatMap((g) => g.items));
  const open = $derived(opened && query.trim() !== "");

  $effect(() => {
    if (autofocus) input.focus({ preventScroll: true });
  });

  function show() {
    opened = true;
    active = 0;
  }

  function choose(entry: SearchEntry | undefined) {
    if (!entry) return;
    opened = false;
    query = "";
    go(entry.href);
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (!open) show();
      else active = Math.min(options.length - 1, active + 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      active = Math.max(0, active - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(options[active]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (open) opened = false;
      else query = "";
    }
  }

  $effect(() => {
    if (open) document.getElementById(`${id}-o${active}`)?.scrollIntoView({ block: "nearest" });
  });
</script>

<div class="finder" class:big>
  <div class="box">
    <input
      bind:this={input}
      bind:value={query}
      class="field"
      type="text"
      role="combobox"
      aria-expanded={open}
      aria-controls="{id}-pop"
      aria-autocomplete="list"
      aria-activedescendant={open && options.length ? `${id}-o${active}` : undefined}
      aria-label={t("ui.search.label")}
      placeholder={t("ui.search.label")}
      autocomplete="off"
      spellcheck="false"
      oninput={show}
      onfocus={() => query.trim() && show()}
      onblur={() => setTimeout(() => (opened = false), 140)}
      {onkeydown}
    />
    <span class="glass" aria-hidden="true"><Icon name="search" /></span>
  </div>
  <div class="pop" id="{id}-pop" role="listbox" aria-label={t("ui.search.results")} hidden={!open}>
    {#if options.length}
      {#each found.groups as group (group.kind)}
        <div role="group" aria-label={t(`ui.search.kind.${group.kind}`)}>
          <p class="kind" aria-hidden="true">{t(`ui.search.kind.${group.kind}`)}</p>
          {#each group.items as entry (entry.href + entry.title)}
            {@const i = options.indexOf(entry)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="option"
              id="{id}-o{i}"
              role="option"
              tabindex="-1"
              aria-selected={i === active}
              onpointerdown={(event) => event.preventDefault()}
              onpointermove={() => (active = i)}
              onclick={() => choose(entry)}
            >
              <Icon name={entry.icon} />
              <b>{#each highlight(entry.title, found.words) as run, n (n)}{#if run.on}<mark>{run.text}</mark>{:else}{run.text}{/if}{/each}</b>
              <small>{entry.meta}</small>
            </div>
          {/each}
        </div>
      {/each}
    {:else}
      <p class="none">{t("ui.search.none", { query: query.trim() })}</p>
    {/if}
  </div>
</div>

<style>
  .finder {
    position: relative;
    width: 100%;
  }
  .big {
    max-width: 460px;
  }
  .box {
    position: relative;
  }
  .field {
    width: 100%;
    height: 36px;
    padding: 0 40px 0 12px;
    border: 1px solid color-mix(in srgb, var(--ink3) 55%, transparent);
    border-radius: 6px;
    background: var(--layer);
    box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--ink3) 55%, transparent);
    caret-color: var(--accent);
    color: var(--ink);
    font: 400 14px/1.35 var(--f);
  }
  .field::placeholder {
    color: var(--ink3);
  }
  .field:hover {
    border-color: var(--ink3);
  }
  .field:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    outline: none;
  }
  .glass {
    position: absolute;
    top: 50%;
    right: 12px;
    color: var(--ink2);
    pointer-events: none;
    transform: translateY(-50%);
  }
  .pop {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 10;
    width: 100%;
    min-width: 340px;
    max-height: min(540px, 64vh);
    padding: 6px 4px 8px 8px;
    overflow: auto;
    border: 1px solid var(--stroke);
    border-radius: 10px;
    background: var(--card);
    box-shadow: var(--shadow-float);
    scrollbar-gutter: stable;
  }
  .pop[hidden] {
    display: none;
  }
  .kind {
    margin: 0;
    padding: 10px 10px 4px;
    color: var(--ink2);
    font: 600 12px/1.3 var(--f);
  }
  .option {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    column-gap: 12px;
    padding: 7px 10px;
    border-radius: 6px;
    cursor: pointer;
  }
  .option[aria-selected="true"] {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .option :global(svg) {
    grid-row: span 2;
    margin-top: 2px;
    color: var(--ink2);
  }
  .option b {
    overflow: hidden;
    font-size: 14px;
    font-weight: 400;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  mark {
    background: none;
    color: inherit;
    font-weight: 700;
  }
  .option small {
    overflow: hidden;
    color: var(--ink2);
    font-size: 12.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    margin: 0;
    padding: 12px 10px;
    color: var(--ink2);
    font-size: 14px;
  }
</style>
