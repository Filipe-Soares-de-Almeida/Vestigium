<script lang="ts">
  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import Spark from "../../components/Spark.svelte";
  import { programRecent } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { formatPercent, formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { ProgramPoint } from "../../lib/types";
  import { formatBytes, formatRate } from "../now/lanes";
  import { NowStream } from "../now/stream.svelte";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";

  let { name }: { name: string } = $props();

  /** The watcher ranks programs every 5 s; the lines follow it. */
  const REFRESH_MS = 5_000;

  const stream = new NowStream();
  const program = $derived(stream.view.frame?.programs?.entries.find((p) => p.name === name) ?? null);

  /** The program at each ranking of the last ten minutes, oldest first. */
  let points = $state<ProgramPoint[]>([]);

  $effect(() => {
    const wanted = name;
    const load = () =>
      void programRecent(wanted).then(
        (rows) => (points = rows),
        () => undefined,
      );
    load();
    const timer = setInterval(() => {
      if (!document.hidden) load();
    }, REFRESH_MS);
    return () => clearInterval(timer);
  });
</script>

{#if !stream.watching}
  <DetailPage fallback={href.programs()} title={name}>
    <p class="text">{t(app.paused ? "ui.now.lede.paused" : app.stopped ? "ui.now.lede.stopped" : "ui.now.lede.unreachable")}</p>
  </DetailPage>
{:else if stream.error !== null}
  <DetailPage fallback={href.programs()} title={name}>
    <ErrorView error={stream.error} heading="h2" onretry={() => stream.retry()} />
  </DetailPage>
{:else if !stream.live.started}
  <DetailPage fallback={href.programs()} title={name}><Loading /></DetailPage>
{:else if program}
  <DetailPage fallback={href.programs()} title={program.name} sub={t("ui.detail.process.sub", { pid: program.pid })}>
    <Facts
      rows={[
        { label: t("ui.now.top.cpu"), value: formatPercent(program.cpu_pct, 1) },
        { label: t("ui.now.top.memory"), value: formatBytes(program.private_bytes) },
        { label: t("ui.now.top.disk"), value: formatRate(program.read_bps + program.write_bps) },
        { label: "GPU", value: formatPercent(program.gpu_pct, 1) },
      ]}
    />
    {#if points.length > 1}
      {@const from = formatTime(points[0].ts_ms)}
      {@const to = t("ui.detail.process.now_word")}
      <Block title={t("ui.detail.process.cpu_recent")}>
        <Spark values={points.map((p) => p.cpu_pct)} at={points.map((p) => p.ts_ms)} label={t("ui.detail.process.cpu_of", { name })} {from} {to} />
      </Block>
      <Block title={t("ui.detail.process.mem_recent")}>
        <Spark values={points.map((p) => p.private_bytes)} at={points.map((p) => p.ts_ms)} label={t("ui.detail.process.mem_of", { name })} {from} {to} />
      </Block>
    {:else}
      <Block title={t("ui.detail.process.cpu_recent")}>
        <p class="p">{t("ui.detail.process.recent_none")}</p>
      </Block>
    {/if}
  </DetailPage>
{:else}
  <DetailPage fallback={href.programs()} title={name}>
    <p class="text">{t("ui.detail.process.gone")}</p>
  </DetailPage>
{/if}

<style>
  .text {
    max-width: 70ch;
    margin: 18px 0 0;
    color: var(--ink2);
    line-height: 1.55;
  }
</style>
