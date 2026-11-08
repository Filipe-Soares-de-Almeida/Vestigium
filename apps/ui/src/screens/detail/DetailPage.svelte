<script lang="ts">
  import type { Snippet } from "svelte";

  import BackLink from "../../components/BackLink.svelte";
  import PageHead from "../../components/PageHead.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { router } from "../../lib/router.svelte";

  let {
    fallback,
    title,
    sub,
    children,
  }: {
    /** Where Back leads when the page was not opened from a list. */
    fallback: string;
    title: string;
    sub?: string;
    children: Snippet;
  } = $props();

  const route = $derived(router.current.screen === "detail" ? router.current : null);
  const to = $derived(route?.from ?? fallback);
  const place = $derived(route?.label ?? t("ui.detail.back_default"));
</script>

<BackLink href={to} label={place} />
<PageHead {title} {sub} />
<div class="cols2 body">{@render children()}</div>

<style>
  /* The facts run across the page; the blocks under them pair up two by two on a wide window. */
  .body > :global(.facts) {
    grid-column: 1 / -1;
  }
</style>
