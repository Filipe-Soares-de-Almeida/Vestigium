<script lang="ts">
  import ToneMark from "./ToneMark.svelte";

  /** What the monitor's silence means for the part of the screen below. */
  let { text }: { text: string } = $props();
</script>

<p class="silent-note" role="status"><ToneMark tone="silent" /><span>{text}</span></p>

<style>
  .silent-note {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin: 0 0 12px;
    padding: 8px 12px;
    border: 1px solid var(--stroke2);
    border-radius: 8px;
    background: var(--row);
    color: var(--ink);
    font-size: 13.5px;
    line-height: 1.45;
  }
  .silent-note :global(.mark) {
    margin-top: 3px;
  }
</style>
