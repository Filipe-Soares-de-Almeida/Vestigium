<script lang="ts">
  import { tick } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  import Icon from "./Icon.svelte";

  // A drop-down list drawn like the search panel; keys follow the select-only combobox pattern.
  let {
    value = $bindable(""),
    options,
    label,
    onchange,
    disabled = false,
    full = false,
    ...rest
  }: {
    value?: string;
    options: { value: string; text: string }[];
    /** Accessible name when no `<label for>` or `aria-labelledby` names the field. */
    label?: string;
    onchange?: (value: string) => void;
    disabled?: boolean;
    /** Takes the width of its container. */
    full?: boolean;
  } & Omit<HTMLButtonAttributes, "value" | "onchange" | "disabled"> = $props();

  const uid = $props.id();
  let open = $state(false);
  /** The option the keys or the pointer are on while the list is open. */
  let active = $state(0);
  let field = $state<HTMLButtonElement | null>(null);
  let list = $state<HTMLUListElement | null>(null);
  /** Where the list shows, fixed to the window so no scrolling container clips it. */
  let place = $state({ left: 0, top: 0, width: 0, up: false });
  let typed = "";
  let typedAt = 0;

  const chosen = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
  const current = $derived(options[chosen] ?? null);

  /** Moves the list to the end of the page, so no container query or transform shifts its fixed position. */
  function portal(node: HTMLElement): { destroy: () => void } {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  async function show(at: number): Promise<void> {
    if (disabled || !field) return;
    active = at;
    const rect = field.getBoundingClientRect();
    place = { left: rect.left, top: rect.bottom + 4, width: rect.width, up: false };
    open = true;
    await tick();
    if (!list) return;
    const height = list.offsetHeight;
    if (rect.bottom + 4 + height > window.innerHeight - 8 && rect.top - 4 - height > 8) {
      place = { ...place, top: rect.top - 4 - height, up: true };
    }
    list.querySelector<HTMLElement>(`[data-i="${at}"]`)?.scrollIntoView({ block: "nearest" });
  }

  function hide(): void {
    open = false;
  }

  function pick(i: number): void {
    hide();
    const next = options[i];
    if (!next || next.value === value) return;
    value = next.value;
    onchange?.(next.value);
  }

  function move(to: number): void {
    active = Math.min(options.length - 1, Math.max(0, to));
    void tick().then(() => list?.querySelector<HTMLElement>(`[data-i="${active}"]`)?.scrollIntoView({ block: "nearest" }));
  }

  /** Jumps to the next option that starts with what was typed in the last half second. */
  function typeahead(key: string): boolean {
    if (key.length !== 1 || key === " ") return false;
    const now = Date.now();
    typed = now - typedAt < 500 ? typed + key.toLowerCase() : key.toLowerCase();
    typedAt = now;
    const from = open ? active : chosen;
    const order = options.map((_, k) => (from + 1 + k) % options.length);
    const hit = order.find((i) => options[i].text.toLowerCase().startsWith(typed));
    if (hit === undefined) return true;
    if (open) move(hit);
    else pick(hit);
    return true;
  }

  function onkeydown(event: KeyboardEvent): void {
    const key = event.key;
    if (!open) {
      if (key === "ArrowDown" || key === "ArrowUp" || key === "Enter" || key === " ") void show(chosen);
      else if (key === "Home") void show(0);
      else if (key === "End") void show(options.length - 1);
      else if (!typeahead(key)) return;
      event.preventDefault();
      return;
    }
    if (key === "ArrowDown") move(active + 1);
    else if (key === "ArrowUp") move(active - 1);
    else if (key === "Home") move(0);
    else if (key === "End") move(options.length - 1);
    else if (key === "PageDown") move(active + 8);
    else if (key === "PageUp") move(active - 8);
    else if (key === "Enter" || key === " ") pick(active);
    else if (key === "Escape") hide();
    else if (key === "Tab") {
      pick(active);
      return;
    } else if (!typeahead(key)) return;
    event.preventDefault();
  }

  // An open list closes when the pointer goes elsewhere, the page scrolls or the window resizes.
  $effect(() => {
    if (!open) return;
    const away = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!field?.contains(target) && !list?.contains(target)) hide();
    };
    const scrolled = (event: Event) => {
      if (!list?.contains(event.target as Node)) hide();
    };
    document.addEventListener("pointerdown", away, true);
    window.addEventListener("scroll", scrolled, true);
    window.addEventListener("resize", hide);
    return () => {
      document.removeEventListener("pointerdown", away, true);
      window.removeEventListener("scroll", scrolled, true);
      window.removeEventListener("resize", hide);
    };
  });
</script>

<span class="select" class:full>
  <button
    {...rest}
    type="button"
    class="field"
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls="{uid}-list"
    aria-activedescendant={open ? `${uid}-o${active}` : undefined}
    {disabled}
    bind:this={field}
    onclick={() => (open ? hide() : void show(chosen))}
    {onkeydown}

  >
    <!-- Every option sits invisibly under the chosen one, so the field is as wide as the longest. -->
    <span class="label">
      <span class="text">{current?.text ?? ""}</span>
      {#each options as option (option.value)}<span class="ghost" aria-hidden="true">{option.text}</span>{/each}
    </span>
    <span class="chev" aria-hidden="true"><Icon name="chevron" /></span>
  </button>
  {#if open}
    <ul
      class="pop"
      class:up={place.up}
      id="{uid}-list"
      role="listbox"
      aria-label={label}
      bind:this={list}
      use:portal
      style:left="{place.left}px"
      style:top="{place.top}px"
      style:min-width="{place.width}px"
    >
      {#each options as option, i (option.value)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          id="{uid}-o{i}"
          class="option"
          class:active={i === active}
          role="option"
          aria-selected={i === chosen}
          data-i={i}
          onpointerdown={(event) => event.preventDefault()}
          onpointermove={() => (active = i)}
          onclick={() => pick(i)}
        >
          {option.text}
        </li>
      {/each}
    </ul>
  {/if}
</span>

<style>
  .select {
    position: relative;
    display: inline-flex;
    min-width: 200px;
  }
  .select.full {
    display: flex;
    width: 100%;
    min-width: 0;
  }
  .field {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    height: 34px;
    padding: 0 40px 0 12px;
    border: 1px solid color-mix(in srgb, var(--ink3) 55%, transparent);
    border-radius: 6px;
    background: var(--field);
    box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--ink3) 55%, transparent);
    color: var(--ink);
    font: 400 14px/1.35 var(--f);
    text-align: left;
    cursor: pointer;
  }
  .field:hover:enabled {
    border-color: var(--ink3);
  }
  .field:focus-visible,
  .field[aria-expanded="true"] {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    outline: none;
  }
  .field:disabled {
    color: var(--ink3);
    cursor: default;
  }
  .field[aria-invalid="true"] {
    border-color: var(--freeze);
  }
  .label {
    display: grid;
    flex: 1 1 auto;
    min-width: 0;
  }
  .label > span {
    grid-area: 1 / 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ghost {
    height: 0;
    visibility: hidden;
  }
  .chev {
    position: absolute;
    top: 50%;
    right: 14px;
    display: grid;
    width: 12px;
    height: 12px;
    margin-top: -6px;
    place-items: center;
    color: var(--ink2);
    pointer-events: none;
    transform: rotate(90deg);
    transition: transform 0.12s ease;
  }
  .field[aria-expanded="true"] .chev {
    transform: rotate(-90deg);
  }
  .chev :global(svg) {
    width: 12px;
    height: 12px;
  }
  .pop {
    position: fixed;
    z-index: 40;
    max-width: min(420px, calc(100vw - 16px));
    max-height: min(320px, 50vh);
    margin: 0;
    padding: 4px;
    overflow: auto;
    border: 1px solid var(--stroke);
    border-radius: 10px;
    background: var(--card);
    box-shadow: var(--shadow-float);
    list-style: none;
  }
  .option {
    position: relative;
    padding: 7px 12px 7px 14px;
    border-radius: 6px;
    color: var(--ink);
    font-size: 14px;
    line-height: 1.35;
    white-space: nowrap;
    cursor: pointer;
  }
  .option.active {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  /* The chosen option carries the same accent bar as the open page in the navigation pane. */
  .option[aria-selected="true"]::before {
    position: absolute;
    top: 9px;
    bottom: 9px;
    left: 4px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
    content: "";
  }
  @media (forced-colors: active) {
    .option.active {
      outline: 1px solid Highlight;
    }
  }
</style>
