<script lang="ts">
  let {
    value = $bindable(""),
    label,
    placeholder,
    type = "text",
    ...rest
  }: {
    value?: string;
    /** Accessible name; the field shows no visible label of its own. */
    label: string;
    placeholder?: string;
    type?: "text" | "search";
    [key: string]: unknown;
  } = $props();
</script>

<input class="field" {type} {placeholder} aria-label={label} bind:value autocomplete="off" spellcheck="false" {...rest} />

<style>
  .field {
    width: 100%;
    height: 36px;
    padding: 0 12px;
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
</style>
