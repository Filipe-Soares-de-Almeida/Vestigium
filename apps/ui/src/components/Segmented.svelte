<script lang="ts" generics="T extends string">
  import { radioGroup } from "../lib/focus";

  interface Option {
    value: T;
    label: string;
    hint?: string;
  }

  let {
    label,
    options,
    value,
    onchange,
    disabled = false,
  }: {
    label: string;
    options: Option[];
    value: T;
    onchange: (value: T) => void;
    disabled?: boolean;
  } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label} aria-disabled={disabled || undefined} use:radioGroup>
  {#each options as option (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      tabindex={option.value === value ? 0 : -1}
      title={option.hint}
      data-focus="{label}:{option.value}"
      {disabled}
      onclick={() => option.value !== value && onchange(option.value)}>{option.label}</button
    >
  {/each}
</div>
