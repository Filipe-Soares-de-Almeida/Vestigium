<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import Spark from "../../components/Spark.svelte";
  import StatusMark, { type StatusKind } from "../../components/StatusMark.svelte";
  import Steps from "../../components/Steps.svelte";
  import { smartHistory } from "../../lib/api";
  import { formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { diskName } from "../../lib/groups";
  import { hwtests } from "../../lib/tests.svelte";
  import type { SmartPoint } from "../../lib/types";
  import { at } from "../dumps/words";
  import { formatDayShort } from "../occurrence/format";
  import { attrName, WATCHED } from "../tests/words";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  let { disk, attr }: { disk: number; attr: number } = $props();

  const device = $derived(hwtests.dto?.disks.find((d) => d.number === disk) ?? null);
  const health = $derived(hwtests.details[disk]?.health ?? null);
  const attribute = $derived(health?.attributes.find((a) => a.id === attr) ?? null);
  const watched = $derived(WATCHED.includes(attr));
  /** A watched count above zero needs a look; the others only inform. */
  const standing = $derived<{ kind: StatusKind; word: string }>(
    !watched
      ? { kind: "info", word: t("ui.detail.smart.state.info") }
      : (attribute?.raw ?? 0) > 0
        ? { kind: "attention", word: t("ui.status.attention") }
        : { kind: "ok", word: t("ui.detail.smart.state.ok") },
  );

  /** The daily readings kept, oldest first; `null` until they arrive. */
  let points = $state<SmartPoint[] | null>(null);

  $effect(() => {
    const name = device?.name;
    if (!name) return;
    void smartHistory(name, attr).then(
      (rows) => (points = rows),
      () => (points = []),
    );
  });

  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });
</script>

{#if attribute && device && health}
  <DetailPage fallback={href.tests(String(disk))} title={attrName(attr)} sub={t("ui.detail.smart.sub", { id: attr, disk: diskName(device) })}>
    <Facts
      rows={[
        { label: t("ui.detail.smart.value"), value: formatNumber(attribute.raw) },
        { label: t("ui.detail.smart.state"), value: mark },
        { label: t("ui.detail.smart.disk"), value: diskName(device) },
        { label: t("ui.detail.smart.read"), value: at(health.read_ms) },
      ]}
    />
    <Block title={t("ui.detail.smart.meaning")}>
      <p class="p">{watched ? t("ui.detail.smart.watched") : t("ui.detail.smart.plain")}</p>
      <p class="note">{t("ui.detail.smart.normalized", { current: attribute.current, worst: attribute.worst })}</p>
    </Block>
    <Block title={t("ui.detail.smart.days")}>
      {#if points && points.length > 1}
        <Spark
          values={points.map((p) => p.raw)}
          at={points.map((p) => p.read_ms)}
          label={t("ui.detail.smart.course", { name: attrName(attr), from: formatNumber(points[0].raw), to: formatNumber(points[points.length - 1].raw) })}
          from={formatDayShort(points[0].read_ms)}
          to={formatDayShort(points[points.length - 1].read_ms)}
          when={formatDayShort}
        />
      {:else if points}
        <p class="p">{t("ui.detail.smart.days_none")}</p>
      {/if}
    </Block>
    {#if standing.kind === "attention"}
      <Block title={t("ui.occurrence.todo.title")}>
        <Steps steps={[{ key: "RunDiskTest", text: t("ui.detail.smart.step"), href: href.tests(String(disk)) }]} />
      </Block>
    {/if}
  </DetailPage>
{:else}
  <DetailPage fallback={href.tests(String(disk))} title={t("ui.detail.smart.title")}>
    <Missing body={device ? t("ui.detail.smart.unread") : undefined} />
  </DetailPage>
{/if}

{#snippet mark()}
  <StatusMark kind={standing.kind} word={standing.word} />
{/snippet}
