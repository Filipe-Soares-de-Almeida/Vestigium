<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { app } from "../../lib/app.svelte";
  import { formatDuration, formatNumber, formatPercent } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { hwtests } from "../../lib/tests.svelte";
  import type { Subject, TestRunDto } from "../../lib/types";
  import { at } from "../dumps/words";
  import { formatBytes } from "../occurrence/format";
  import { benchModeName, benchOf, checkOf, kindOf, outcomeMark, quickOf, surfaceOf } from "../tests/words";
  import { statusOf } from "../tests/status";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  let { id }: { id: number } = $props();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const run = $derived<TestRunDto | null>(hwtests.dto?.runs.find((r) => r.id === id) ?? null);
  const quick = $derived(run ? quickOf(run) : null);
  const surface = $derived(run ? surfaceOf(run) : null);
  const check = $derived(run ? checkOf(run) : null);
  const bench = $derived(run ? benchOf(run) : null);

  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });

  const where = (s: Subject) =>
    s === "Pc" ? t("ui.detail.pc") : "Disk" in s ? s.Disk.replace(/^\d+\s*/, "") || s.Disk : s.Volume.replace(/\\$/, "");
  const device = (s: Subject) => (s === "Pc" ? href.tests("memory") : "Disk" in s ? href.tests(s.Disk.split(" ")[0]) : href.tests());
</script>

{#if hwtests.dto === null}
  <DetailPage fallback={href.tests()} title={t("ui.tests.title")}><span></span></DetailPage>
{:else if run}
  <DetailPage fallback={device(run.subject)} title={t(`ui.detail.test.${kindOf(run)}`)} sub={where(run.subject)}>
    <Facts
      rows={[
        { label: t("ui.detail.outcome"), value: outcome },
        { label: t("ui.detail.started"), value: at(run.started_ms) },
        { label: t("ui.detail.duration"), value: formatDuration(run.ended_ms - run.started_ms) },
      ]}
    />

    {#if quick}
      <Block title={t("ui.detail.result")} first>
        <p class="line">
          {t("ui.detail.quick.tested", { tested: formatBytes(quick.tested_bytes), total: formatBytes(quick.total_bytes), share: formatPercent((quick.tested_bytes / Math.max(1, quick.total_bytes)) * 100) })}
        </p>
        <p class="line">{t("ui.detail.quick.rounds", { rounds: quick.rounds, passes: quick.passes })}</p>
        <p class="line">{quick.error_count ? t("ui.detail.quick.errors", { count: formatNumber(quick.error_count) }) : t("ui.detail.quick.clean")}</p>
        {#if quick.failure}<p class="line">{quick.failure}</p>{/if}
      </Block>
      {#if quick.errors.length}
        <Block title={t("ui.detail.quick.errors_title")}>
          <table>
            <thead><tr><th scope="col">{t("ui.detail.col.offset")}</th><th scope="col">{t("ui.detail.col.pattern")}</th><th scope="col" class="n">{t("ui.detail.col.expected")}</th><th scope="col" class="n">{t("ui.detail.col.read")}</th></tr></thead>
            <tbody>
              {#each quick.errors as e, i (i)}
                <tr><td>{formatBytes(e.offset)}</td><td>{e.pattern}</td><td class="n mono">{e.expected.toString(16)}</td><td class="n mono">{e.actual.toString(16)}</td></tr>
              {/each}
            </tbody>
          </table>
        </Block>
      {/if}
    {:else if surface}
      <Block title={t("ui.detail.result")} first>
        <p class="line">{t("ui.detail.surface.range", { from: formatBytes(surface.from_bytes), to: formatBytes(surface.to_bytes), total: formatBytes(surface.disk_bytes) })}</p>
        <p class="line">{surface.unreadable_blocks ? t("ui.detail.surface.unreadable", { count: formatNumber(surface.unreadable_blocks) }) : t("ui.detail.surface.clean")}</p>
        {#if surface.failure}<p class="line">{surface.failure}</p>{/if}
      </Block>
      {#if surface.worst.length && level !== "Simple"}
        <Block title={t("ui.detail.surface.worst")}>
          <table>
            <thead><tr><th scope="col">{t("ui.detail.col.offset")}</th><th scope="col" class="n">{t("ui.detail.col.ms")}</th></tr></thead>
            <tbody>
              {#each surface.worst.slice(0, 20) as b, i (i)}
                <tr><td>{formatBytes(b.offset)}</td><td class="n">{formatNumber(b.ms)}</td></tr>
              {/each}
            </tbody>
          </table>
        </Block>
      {/if}
    {:else if check}
      <Block title={t("ui.detail.result")} first>
        <p class="line">{t(`ui.detail.check.${check.result}`)}</p>
      </Block>
      {#if check.output && level !== "Simple"}
        <Block title={t("ui.detail.check.output")}><pre>{check.output}</pre></Block>
      {/if}
    {:else if bench}
      <Block title={t("ui.detail.result")} first>
        <table>
          <thead><tr><th scope="col">{t("ui.detail.col.mode")}</th><th scope="col" class="n">MB/s</th><th scope="col" class="n">IOPS</th><th scope="col" class="n">{t("ui.detail.col.ms")}</th></tr></thead>
          <tbody>
            {#each bench.results as r (r.mode)}
              <tr><td>{benchModeName(r.mode, level)}</td><td class="n">{formatNumber(r.mb_per_s)}</td><td class="n">{formatNumber(r.iops)}</td><td class="n">{formatNumber(r.max_ms)}</td></tr>
            {/each}
          </tbody>
        </table>
        {#if bench.outside_io}<p class="line">{t("ui.detail.bench.outside")}</p>{/if}
      </Block>
    {/if}
  </DetailPage>
{:else}
  <DetailPage fallback={href.tests()} title={t("ui.tests.title")}><Missing /></DetailPage>
{/if}

{#snippet outcome()}
  {#if run}
    {@const st = statusOf(outcomeMark(run))}
    <StatusMark kind={st.kind} word={t(`ui.detail.outcome.${run.outcome}`)} />
  {/if}
{/snippet}

<style>
  .line {
    max-width: 70ch;
    margin: 0 0 6px;
    line-height: 1.5;
  }
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
  .mono,
  pre {
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12.5px;
  }
  pre {
    max-height: 320px;
    margin: 0;
    overflow: auto;
    white-space: pre-wrap;
    color: var(--ink2);
  }
</style>
