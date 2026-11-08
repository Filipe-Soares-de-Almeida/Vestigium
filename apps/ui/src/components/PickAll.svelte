<script lang="ts" generics="T">
  import Checkbox from "./Checkbox.svelte";

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
</script>

<Checkbox checked={all} indeterminate={some} disabled={ids.length === 0} onchange={() => onchange(!all)}>
  <span class="sr">{label}</span>
</Checkbox>
