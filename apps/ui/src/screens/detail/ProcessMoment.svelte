<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import Spark from "../../components/Spark.svelte";
  import { occurrence } from "../../lib/api";
  import { formatClock, formatPercent, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { here, href } from "../../lib/router.svelte";
  import type { OccurrenceDto } from "../../lib/types";
  import { formatBytes, formatRate } from "../occurrence/format";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  /** One process as an occurrence's rankings caught it: its peaks and its course over the window. */
  let { name, occurrenceId, pid }: { name: string; occurrenceId: number; pid: number } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<OccurrenceDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);

  /** The process in each ranking of the window that holds it, oldest first. */
  const readings = $derived(
    (dto?.window.tops ?? []).flatMap((top) =>
      top.entries.filter((e) => e.pid === pid && e.name === name).map((e) => ({ ts: top.ts_ms, e })),
    ),
  );
  const peak = $derived(
    readings.length
      ? {
          cpu: Math.max(...readings.map((r) => r.e.cpu_pct)),
          mem: Math.max(...readings.map((r) => r.e.private_bytes)),
          io: Math.max(...readings.map((r) => r.e.read_bps + r.e.write_bps)),
        }
      : null,
  );
  const sub = $derived(dto ? t("ui.detail.process.at", { time: formatTime(dto.start_ms) }) : undefined);

  async function load(): Promise<void> {
    failure = null;
    dto = undefined;
    try {
      dto = await occurrence(occurrenceId);
    } catch (error) {
      failure = error;
    }
  }

  onMount(() => {
    void load();
  });
</script>

{#if failure !== null}
  <DetailPage fallback={href.occurrence(occurrenceId)} title={name}>
    <ErrorView error={failure} heading="h2" onretry={load} />
  </DetailPage>
{:else if dto === undefined}
  <DetailPage fallback={href.occurrence(occurrenceId)} title={name}><Loading /></DetailPage>
{:else if dto && peak}
  <DetailPage fallback={href.occurrence(occurrenceId)} title={name} {sub}>
    <Facts
      rows={[
        { label: t("ui.detail.process.peak_cpu"), value: formatPercent(peak.cpu, 1) },
        { label: t("ui.detail.process.peak_mem"), value: formatBytes(peak.mem) },
        { label: t("ui.detail.process.peak_disk"), value: formatRate(peak.io) },
        { label: t("ui.detail.process.pid"), value: String(pid) },
      ]}
    />
    {#if readings.length > 1}
      {@const from = formatClock(readings[0].ts)}
      {@const to = formatClock(readings[readings.length - 1].ts)}
      <Block title={t("ui.detail.process.cpu_window")}>
        <Spark values={readings.map((r) => r.e.cpu_pct)} at={readings.map((r) => r.ts)} label={t("ui.detail.process.cpu_of", { name })} {from} {to} format={(v) => formatPercent(v, 1)} floor={0} ceiling={100} ceilingLabel={formatPercent(100)} />
      </Block>
      <Block title={t("ui.detail.process.mem_window")}>
        <Spark values={readings.map((r) => r.e.private_bytes)} at={readings.map((r) => r.ts)} label={t("ui.detail.process.mem_of", { name })} {from} {to} format={formatBytes} floor={0} ceiling={dto.window.machine.mem_total_bytes || undefined} ceilingLabel={dto.window.machine.mem_total_bytes ? formatBytes(dto.window.machine.mem_total_bytes) : undefined} />
      </Block>
    {:else}
      <Block title={t("ui.detail.process.cpu_window")}>
        <p class="p">{t("ui.detail.process.one_reading")}</p>
      </Block>
    {/if}
    <p class="more">
      <a href={href.detail("process", name, { from: here(), label: name })}>{t("ui.detail.process.see_live")}</a>
    </p>
  </DetailPage>
{:else}
  <DetailPage fallback={href.occurrence(occurrenceId)} title={name}><Missing /></DetailPage>
{/if}

<style>
  .more {
    grid-column: 1 / -1;
  }
</style>
