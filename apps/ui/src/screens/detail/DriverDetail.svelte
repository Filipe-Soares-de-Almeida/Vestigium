<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import { captures } from "../../lib/capture.svelte";
  import { formatClock, formatNumber } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { CaptureDto, RoutineStats } from "../../lib/types";
  import { at } from "../dumps/words";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  let { captureId, module }: { captureId: number; module: string } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<CaptureDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);

  const driver = $derived(dto?.drivers.find((d) => d.module === module) ?? null);
  /** The page lists each driver's five longest kept calls, longest first. */
  const calls = $derived((dto?.longest ?? []).filter((c) => c.module === module).slice(0, 5));

  /** Microseconds as the report prints them: "840 µs" or "1.24 ms". */
  const us = (v: number) => (v >= 1000 ? `${formatNumber(v / 1000)} ms` : `${formatNumber(v)} µs`);
  const row = (kind: string, s: RoutineStats) => ({ kind, count: s.count, max: s.max_us, total: s.total_us });

  async function load(): Promise<void> {
    failure = null;
    dto = undefined;
    try {
      dto = await captures.get(captureId);
    } catch (error) {
      failure = error;
    }
  }

  onMount(() => {
    void load();
  });
</script>

{#if failure !== null}
  <DetailPage fallback={href.capture(captureId)} title={module}><ErrorView error={failure} heading="h2" onretry={load} /></DetailPage>
{:else if dto === undefined}
  <DetailPage fallback={href.capture(captureId)} title={module}><Loading /></DetailPage>
{:else if driver}
  <DetailPage fallback={href.capture(captureId)} title={driver.module} sub={at(dto!.start_ms)}>
    <Facts
      rows={[
        { label: t("ui.detail.driver.longest"), value: us(Math.max(driver.dpc.max_us, driver.isr.max_us)) },
        { label: t("ui.detail.driver.calls"), value: formatNumber(driver.dpc.count + driver.isr.count) },
        { label: t("ui.detail.driver.total"), value: us(driver.dpc.total_us + driver.isr.total_us) },
      ]}
    />
    <Block title={t("ui.detail.driver.routines")}>
      <table>
        <thead>
          <tr>
            <th scope="col">{t("ui.detail.col.kind")}</th>
            <th scope="col" class="n">{t("ui.detail.col.calls")}</th>
            <th scope="col" class="n">{t("ui.detail.col.longest")}</th>
            <th scope="col" class="n">{t("ui.detail.col.total")}</th>
          </tr>
        </thead>
        <tbody>
          {#each [row("DPC", driver.dpc), row("ISR", driver.isr)] as r (r.kind)}
            <tr><td>{r.kind}</td><td class="n">{formatNumber(r.count)}</td><td class="n">{us(r.max)}</td><td class="n">{us(r.total)}</td></tr>
          {/each}
        </tbody>
      </table>
      <p class="note">{t("ui.detail.driver.note")}</p>
    </Block>
    <Block title={t("ui.detail.driver.long_calls")}>
      {#if calls.length}
        <table>
          <thead>
            <tr>
              <th scope="col">{t("ui.detail.driver.col.rank")}</th>
              <th scope="col" class="n">{t("ui.detail.duration")}</th>
              <th scope="col">{t("ui.detail.col.kind")}</th>
              <th scope="col" class="n">{t("ui.detail.col.core")}</th>
              <th scope="col">{t("ui.detail.event.time")}</th>
            </tr>
          </thead>
          <tbody>
            {#each calls as c, i (i)}
              <tr>
                <td>{i + 1}</td>
                <td class="n">{us(c.us)}</td>
                <td>{c.kind === "Dpc" ? "DPC" : "ISR"}</td>
                <td class="n">{c.core}</td>
                <td>{formatClock(c.ts_ms)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="empty">{t("ui.detail.driver.no_calls")}</p>
      {/if}
    </Block>
  </DetailPage>
{:else}
  <DetailPage fallback={href.capture(captureId)} title={module}><Missing /></DetailPage>
{/if}

<style>
  table {
    width: 100%;
    max-width: 640px;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  th,
  td {
    padding: 8px;
    border-bottom: 1px solid var(--stroke2);
    text-align: left;
  }
  thead th {
    color: var(--ink2);
    font-weight: 600;
  }
  .n {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .note {
    max-width: 70ch;
    margin: 12px 0 0;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
</style>
