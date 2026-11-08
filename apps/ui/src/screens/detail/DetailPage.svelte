<script lang="ts">
  import type { Snippet } from "svelte";

  import BackLink from "../../components/BackLink.svelte";
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

<BackLink href={to} label={t("ui.common.back_to", { place })} />
<header class="head">
  <h1>{title}</h1>
  {#if sub}<p class="sub">{sub}</p>{/if}
</header>
{@render children()}

<style>
  .head {
    margin-bottom: 4px;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
</style>
