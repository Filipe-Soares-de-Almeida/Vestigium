<script lang="ts">
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import SelectField from "../components/SelectField.svelte";
  import TextField from "../components/TextField.svelte";
  import { app } from "../lib/app.svelte";
  import { formatPercent } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { here, href } from "../lib/router.svelte";
  import { formatBytes, formatRate } from "./now/lanes";
  import { type ProgramSort, sortPrograms } from "./now/sort";
  import { NowStream } from "./now/stream.svelte";

  /** Rows the table shows at most, after the filter and the sort. */
  const SHOWN = 100;

  const stream = new NowStream();
  const programs = $derived(stream.view.frame?.programs ?? null);
  const total = $derived(programs?.entries.length ?? 0);
  let sort = $state<ProgramSort>("cpu");
  let query = $state("");

  const ranked = $derived(sortPrograms(programs?.entries ?? [], sort).slice(0, SHOWN));
  const rows = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    return needle ? ranked.filter((p) => p.name.toLowerCase().includes(needle)) : ranked;
  });
  const options = $derived([
    { value: "cpu", text: t("ui.now.top.cpu") },
    { value: "memory", text: t("ui.now.top.memory") },
    { value: "disk", text: t("ui.now.top.disk") },
  ]);
  const last = $derived(stream.view.frame?.latest ?? null);

  relatedLinks(() => [
    { label: t("ui.related.live_now"), href: href.activity("now") },
    ...(last ? [{ label: t("ui.related.last_freeze"), href: href.occurrence(last.id) }] : []),
  ]);
</script>

{#if !stream.watching}
  <PageHead
    title={t("ui.item.programs")}
    sub={t(app.paused ? "ui.now.lede.paused" : app.stopped ? "ui.now.lede.stopped" : "ui.now.lede.unreachable")}
  />
{:else if stream.error !== null}
  <PageHead title={t("ui.item.programs")} />
  <ErrorView error={stream.error} onretry={() => stream.retry()} />
{:else if !stream.live.started}
  <PageHead title={t("ui.item.programs")} />
  <Loading />
{:else}
  <PageHead
    title={t("ui.item.programs")}
    sub={programs ? t("ui.now.all.sub", { total, shown: ranked.length }) : t("ui.now.top.waiting")}
  />
  <div class="tools">
    <div class="filter">
      <TextField bind:value={query} label={t("ui.now.all.search")} placeholder={t("ui.now.all.search")} />
    </div>
    <label class="inl">
      <span>{t("ui.now.sort")}</span>
      <SelectField value={sort} {options} label={t("ui.now.sort")} onchange={(value) => (sort = value as ProgramSort)} />
    </label>
    {#if stream.paused}<span class="paused">{t("ui.now.all.paused")}</span>{/if}
  </div>
  <div class="wide">
    <table class="tbl">
      <thead>
        <tr>
          <th scope="col">{t("ui.now.top.program")}</th>
          <th scope="col" class="num">{t("ui.now.top.cpu")}</th>
          <th scope="col" class="num">{t("ui.now.top.memory")}</th>
          <th scope="col" class="num">{t("ui.now.top.disk")}</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.pid)}
          <tr>
            <td class="nm" title={row.name}>
              <a class="row-a" href={href.detail("process", row.name, { from: here(), label: t("ui.item.programs") })}>{row.name}</a>
            </td>
            <td class="num">{formatPercent(row.cpu_pct, 1)}</td>
            <td class="num">{formatBytes(row.private_bytes)}</td>
            <td class="num">{formatRate(row.read_bps + row.write_bps)}</td>
          </tr>
        {:else}
          <tr><td colspan="4" class="empty-cell">{t(programs ? "ui.now.all.none" : "ui.now.top.waiting")}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  <p class="note">{t("ui.now.all.count", { shown: rows.length, total: ranked.length })}</p>
{/if}

<style>
  .filter {
    width: 300px;
  }
  .paused {
    color: var(--ink);
    font-size: 13px;
    font-weight: 600;
  }
</style>
