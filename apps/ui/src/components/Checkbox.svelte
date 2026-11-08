<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    checked = $bindable(false),
    indeterminate = false,
    disabled = false,
    onchange,
    children,
  }: {
    checked?: boolean;
    /** Shows a dash while only some of a group are ticked. */
    indeterminate?: boolean;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
    children: Snippet;
  } = $props();

  /** `indeterminate` is a property, not an attribute, so it is set on the element. */
  function mixed(node: HTMLInputElement, on: boolean): { update: (on: boolean) => void } {
    node.indeterminate = on;
    return { update: (next) => (node.indeterminate = next) };
  }
</script>

<label class="check" class:disabled>
  <input type="checkbox" bind:checked {disabled} use:mixed={indeterminate} onchange={() => onchange?.(checked)} />
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
    background: var(--field);
  }
  .check:hover .box {
    border-color: var(--ink);
  }
  input:checked + .box,
  input:indeterminate + .box {
    border-color: var(--accent);
    background: var(--accent);
  }
  input:indeterminate:not(:checked) + .box::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    width: 10px;
    height: 2px;
    border-radius: 2px;
    background: var(--on-accent);
    transform: translate(-50%, -50%);
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
