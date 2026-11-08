<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    checked = $bindable(false),
    disabled = false,
    onchange,
    children,
  }: {
    checked?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    children: Snippet;
  } = $props();
</script>

<label class="check" class:disabled>
  <input type="checkbox" bind:checked {disabled} onchange={() => onchange?.(checked)} />
  <span class="box" aria-hidden="true"></span>
  <span class="text">{@render children()}</span>
</label>

<style>
  .check {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    font-size: 14px;
    line-height: 1.5;
    cursor: pointer;
  }
  input {
    position: absolute;
    top: 1px;
    left: 0;
    width: 20px;
    height: 20px;
    margin: 0;
    opacity: 0;
    cursor: inherit;
  }
  .box {
    position: relative;
    flex: none;
    width: 20px;
    height: 20px;
    margin-top: 1px;
    border: 1.5px solid var(--ink2);
    border-radius: 6px;
    background: var(--layer);
  }
  .check:hover .box {
    border-color: var(--ink);
  }
  input:checked + .box {
    border-color: var(--accent);
    background: var(--accent);
  }
  input:checked + .box::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 6px;
    height: 11px;
    border: solid var(--on-accent);
    border-width: 0 2px 2px 0;
    transform: translate(-50%, -62%) rotate(45deg);
  }
  input:focus-visible + .box {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }
  input:checked ~ .text {
    color: var(--ink2);
  }
  .disabled {
    cursor: default;
    opacity: 0.55;
  }
</style>
