<script lang="ts" generics="T">
  /** One box that ticks or clears every item of a group; it shows a dash while only some are ticked. */
  let {
    ids,
    chosen,
    label,
    onchange,
  }: {
    ids: T[];
    chosen: T[];
    /** What the box does, for a screen reader. */
    label: string;
    onchange: (on: boolean) => void;
  } = $props();

  const count = $derived(ids.filter((id) => chosen.includes(id)).length);
  const all = $derived(ids.length > 0 && count === ids.length);
  const some = $derived(count > 0 && !all);

  /** `indeterminate` is a property, not an attribute, so it is set on the element. */
  function mixed(node: HTMLInputElement, on: boolean): { update: (on: boolean) => void } {
    node.indeterminate = on;
    return { update: (next) => (node.indeterminate = next) };
  }
</script>

<span class="all">
  <input
    type="checkbox"
    class="pick"
    checked={all}
    disabled={ids.length === 0}
    aria-label={label}
    use:mixed={some}
    onchange={() => onchange(!all)}
  />
</span>

<style>
  .all {
    display: grid;
    flex: none;
    place-items: center;
    width: 22px;
  }
  .pick {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: 0;
    border: 1.5px solid var(--ink3);
    border-radius: 6px;
    background: var(--card);
    appearance: none;
    cursor: pointer;
  }
  .pick:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .pick:checked,
  .pick:indeterminate {
    border-color: var(--accent);
    background: var(--accent);
  }
  .pick:checked::after {
    content: "";
    width: 9px;
    height: 5px;
    border-bottom: 2px solid var(--on-accent);
    border-left: 2px solid var(--on-accent);
    transform: rotate(-45deg) translate(1px, -1px);
  }
  .pick:indeterminate::after {
    content: "";
    width: 10px;
    height: 2px;
    border-radius: 1px;
    background: var(--on-accent);
  }
  @media (forced-colors: active) {
    .pick:checked,
    .pick:indeterminate {
      forced-color-adjust: none;
      background: Highlight;
      border-color: Highlight;
    }
  }
</style>
