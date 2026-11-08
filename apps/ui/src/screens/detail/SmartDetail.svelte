<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { hwtests } from "../../lib/tests.svelte";
  import { attrName, WATCHED } from "../tests/words";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  let { disk, attr }: { disk: number; attr: number } = $props();

  const device = $derived(hwtests.dto?.disks.find((d) => d.number === disk) ?? null);
  const attribute = $derived(hwtests.details[disk]?.health?.attributes.find((a) => a.id === attr) ?? null);

  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });
</script>

{#if attribute}
  <DetailPage fallback={href.tests(String(disk))} title={attrName(attr)} sub={t("ui.detail.smart.sub", { id: attr })}>
    <Facts
      rows={[
        { label: t("ui.detail.smart.current"), value: formatNumber(attribute.current) },
        { label: t("ui.detail.smart.worst"), value: formatNumber(attribute.worst) },
        { label: t("ui.detail.smart.raw"), value: formatNumber(attribute.raw) },
      ]}
    />
    <Block title={t("ui.detail.smart.meaning")}>
      <p class="text">{WATCHED.includes(attr) ? t("ui.detail.smart.watched") : t("ui.detail.smart.plain")}</p>
      <p class="text note">{t("ui.detail.smart.latest_only")}</p>
    </Block>
  </DetailPage>
{:else}
  <DetailPage fallback={href.tests(String(disk))} title={t("ui.detail.smart.title")}>
    <Missing body={device ? t("ui.detail.smart.unread") : undefined} />
  </DetailPage>
{/if}

<style>
  .text {
    max-width: 70ch;
    margin: 0 0 8px;
    line-height: 1.55;
  }
  .note {
    color: var(--ink2);
    font-size: 13.5px;
  }
</style>
