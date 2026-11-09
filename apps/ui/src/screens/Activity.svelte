<script lang="ts">
  import type { ActivityTab } from "../lib/router.svelte";
  import Now from "./Now.svelte";
  import Today from "./Today.svelte";

  let { tab }: { tab: ActivityTab } = $props();
</script>

{#if tab === "now"}
  <Now />
{:else}
  <Today />
{/if}
