<script lang="ts">
  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import Spark from "../../components/Spark.svelte";
  import { processFacts, programMarks, programRecent } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { formatNumber, formatPercent, formatTime } from "../../lib/format";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { ProcessFacts, ProgramMarks, ProgramPoint, RunAs, Signature } from "../../lib/types";
  import { at } from "../dumps/words";
  import { formatBytes, formatRate } from "../now/lanes";
  import { NowStream } from "../now/stream.svelte";
  import { dayStart } from "../today/day";
  import EpisodeRow from "../today/EpisodeRow.svelte";
  import OccurrenceRow from "../today/OccurrenceRow.svelte";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";

  let { name }: { name: string } = $props();

  /** The watcher ranks programs every 5 s; the lines follow it. */
  const REFRESH_MS = 5_000;

  const stream = new NowStream();
  const program = $derived(stream.view.frame?.programs?.entries.find((p) => p.name === name) ?? null);
  const level = $derived(app.settings?.settings.detail_level ?? "Simple");

  /** The program at each ranking of the last ten minutes, oldest first. */
  let points = $state<ProgramPoint[]>([]);
  /** Its file, user and processes, read once per process. */
  let facts = $state<ProcessFacts | null>(null);
  /** Today's occurrences and overload periods that involve it. */
  let marks = $state<ProgramMarks | null>(null);

  const pid = $derived(program?.pid ?? null);
  /** A program under the Windows folder is part of Windows. */
  const windows = $derived(facts?.path ? /^[a-z]:\\windows\\/i.test(facts.path) : false);
  const moments = $derived(
    marks
      ? [
          ...marks.occurrences.map((o) => ({ at: o.start_ms, key: `o${o.id}`, occurrence: o, episode: null })),
          ...marks.episodes.map((e) => ({ at: e.start_ms, key: `e${e.start_ms}`, occurrence: null, episode: e })),
        ].sort((a, b) => b.at - a.at)
      : [],
  );

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

  $effect(() => {
    const wanted = name;
    const now = Date.now();
    void programMarks(wanted, dayStart(now), now).then(
      (found) => (marks = found),
      () => (marks = { occurrences: [], episodes: [] }),
    );
  });

  $effect(() => {
    if (pid === null) return;
    const wanted = name;
    void processFacts(pid, wanted).then(
      (found) => (facts = found),
      () => undefined,
    );
  });

  const runAsText = (who: RunAs): string =>
    typeof who === "string" ? t(`ui.detail.process.run_as.${who}`) : t(who.CurrentUser.elevated ? "ui.detail.process.run_as.admin" : "ui.detail.process.run_as.CurrentUser");
  const signatureText = (s: Signature): string =>
    typeof s === "string" ? t(`ui.detail.process.signature.${s}`) : s.Valid.signer || t("ui.detail.process.signature.Valid");
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
  {@const many = facts?.processes ?? 1}
  <DetailPage
    fallback={href.programs()}
    title={program.name}
    sub={`${t(windows ? "ui.detail.process.windows" : "ui.detail.process.running")}${many > 1 ? ` · ${tn("ui.detail.process.processes", many)}` : ""}`}
  >
    <Facts
      rows={[
        { label: t("ui.now.top.cpu"), value: formatPercent(program.cpu_pct, 1) },
        { label: t("ui.now.top.memory"), value: formatBytes(program.private_bytes) },
        { label: t("ui.now.top.disk"), value: formatRate(program.read_bps + program.write_bps) },
        { label: "GPU", value: formatPercent(program.gpu_pct, 1) },
        { label: t("ui.detail.process.pid"), value: String(program.pid) },
        { label: t("ui.detail.process.started"), value: facts?.started_ms != null ? at(facts.started_ms) : "—" },
        { label: t("ui.detail.process.threads"), value: facts?.threads != null ? formatNumber(facts.threads) : "—" },
      ]}
    />
    {#if points.length > 1}
      {@const from = formatTime(points[0].ts_ms)}
      {@const to = t("ui.detail.process.now_word")}
      <Block title={t("ui.detail.process.cpu_recent")}>
        <Spark values={points.map((p) => p.cpu_pct)} at={points.map((p) => p.ts_ms)} label={t("ui.detail.process.cpu_of", { name })} {from} {to} format={(v) => formatPercent(v, 1)} />
      </Block>
      <Block title={t("ui.detail.process.mem_recent")}>
        <Spark values={points.map((p) => p.private_bytes)} at={points.map((p) => p.ts_ms)} label={t("ui.detail.process.mem_of", { name })} {from} {to} format={formatBytes} />
      </Block>
    {:else}
      <Block title={t("ui.detail.process.cpu_recent")}>
        <p class="p">{t("ui.detail.process.recent_none")}</p>
      </Block>
    {/if}
    <Block title={t("ui.detail.process.file")}>
      {#if facts}
        <dl class="kv">
          <div><dt>{t("ui.detail.process.path")}</dt><dd>{#if facts.path}<code class="code">{facts.path}</code>{:else}{t("ui.detail.process.path_closed")}{/if}</dd></div>
          <div><dt>{t("ui.detail.process.signature")}</dt><dd>{signatureText(facts.signature)}</dd></div>
          <div><dt>{t("ui.detail.process.run_as")}</dt><dd>{runAsText(facts.run_as)}</dd></div>
        </dl>
      {:else}
        <Loading />
      {/if}
    </Block>
    <Block title={t("ui.detail.process.today")}>
      {#if marks === null}
        <Loading />
      {:else if moments.length}
        <ul class="rows">
          {#each moments as m (m.key)}
            <li>
              {#if m.occurrence}<OccurrenceRow o={m.occurrence} {level} />{:else if m.episode}<EpisodeRow e={m.episode} {level} />{/if}
            </li>
          {/each}
        </ul>
      {:else}
        <p class="empty">{t("ui.detail.process.today_none")}</p>
      {/if}
    </Block>
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
  .kv {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 8px 32px;
    margin: 0;
    font-size: 14px;
  }
  .kv div {
    display: contents;
  }
  .kv dt {
    color: var(--ink2);
  }
  .kv dd {
    min-width: 0;
    margin: 0;
    overflow-wrap: anywhere;
  }
  .rows {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
