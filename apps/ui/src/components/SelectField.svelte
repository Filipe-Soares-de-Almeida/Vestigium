<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(""),
    options,
    label,
    onchange,
  }: {
    value?: string;
    options: { value: string; text: string }[];
    /** Accessible name; the list shows no visible label of its own. */
    label: string;
    onchange?: (value: string) => void;
  } = $props();
</script>

<span class="select">
  <select bind:value aria-label={label} onchange={() => onchange?.(value)}>
    {#each options as option (option.value)}
      <option value={option.value}>{option.text}</option>
    {/each}
  </select>
  <span class="chev" aria-hidden="true"><Icon name="chevron" /></span>
</span>

<style>
  .select {
    position: relative;
    display: inline-flex;
    min-width: 200px;
  }
  select {
    width: 100%;
    height: 34px;
    padding: 0 36px 0 12px;
    border: 1px solid color-mix(in srgb, var(--ink3) 55%, transparent);
    border-radius: 6px;
    appearance: none;
    background: var(--layer);
    box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--ink3) 55%, transparent);
    color: var(--ink);
    font: 400 14px/1.35 var(--f);
  }
  select:hover {
    border-color: var(--ink3);
  }
  select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
    outline: none;
  }
  option {
    background: var(--card);
    color: var(--ink);
  }
  .chev {
    position: absolute;
    top: 50%;
    right: 12px;
    margin-top: -6px;
    color: var(--ink2);
    pointer-events: none;
    transform: rotate(90deg);
  }
</style>
