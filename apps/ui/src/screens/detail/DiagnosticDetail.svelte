<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import Steps from "../../components/Steps.svelte";
  import { app } from "../../lib/app.svelte";
  import { formatDuration } from "../../lib/format";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { hwtests } from "../../lib/tests.svelte";
  import { at } from "../dumps/words";
  import { formatBytes } from "../occurrence/format";
  import { statusOf } from "../tests/status";
  import { diagnosticCovered, diagnosticWord } from "../tests/words";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  /** One run of the Windows Memory Diagnostic, named by the System log record of its first event. */
  let { recordId }: { recordId: number } = $props();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const run = $derived(hwtests.dto?.diagnostics.find((d) => d.record_id === recordId) ?? null);
  const word = $derived(run ? diagnosticWord(run) : null);

  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });

  function retry(): void {
    hwtests.error = null;
    void hwtests.load();
  }
</script>

{#if hwtests.dto === null && hwtests.error !== null}
  <DetailPage fallback={href.tests("memory")} title={t("ui.tests.diag.title")}><ErrorView error={hwtests.error} heading="h2" onretry={retry} /></DetailPage>
{:else if hwtests.dto === null}
  <DetailPage fallback={href.tests("memory")} title={t("ui.tests.diag.title")}><span></span></DetailPage>
{:else if run && word}
  <DetailPage fallback={href.tests("memory")} title={t("ui.tests.diag.title")} sub={at(run.ts_ms)}>
    <Facts
      rows={[
        { label: t("ui.detail.device"), value: t("ui.group.memory") },
        { label: t("ui.detail.outcome"), value: outcome },
        { label: t("ui.detail.duration"), value: run.detail ? formatDuration(run.detail.duration_s * 1000) : t("ui.detail.unknown") },
        { label: t("ui.detail.run_by"), value: t("ui.detail.diag.run_by") },
      ]}
    />
    <Block title={t("ui.detail.diag.what_title")}>
      <p class="p">{t("ui.detail.diag.what")}</p>
    </Block>
    <Block title={t("ui.detail.result")}>
      <ul class="ev-l">
        <li>{diagnosticCovered(run, level)}</li>
        {#if run.detail}
          <li>{run.detail.bad_pages ? tn("ui.tests.diag.bad_pages", run.detail.bad_pages) : t("ui.detail.diag.no_bad")}</li>
          <li>{t("ui.detail.diag.size", { size: formatBytes(run.detail.memory_mb * 1048576) })}</li>
        {:else if run.result === "Errors"}
          <li>{t("ui.tests.diag.errors")}</li>
        {/if}
      </ul>
      {#if run.result === "Errors"}
        <h3 class="h3">{t("ui.occurrence.todo.title")}</h3>
        <Steps
          steps={[
            { key: "save", text: t("ui.detail.diag.step.save") },
            { key: "swap", text: t("ui.detail.diag.step.swap") },
          ]}
        />
      {/if}
    </Block>
  </DetailPage>
{:else}
  <DetailPage fallback={href.tests("memory")} title={t("ui.tests.diag.title")}><Missing body={t("ui.detail.diag.gone")} /></DetailPage>
{/if}

{#snippet outcome()}
  {#if word}
    {@const st = statusOf(word.mark)}
    <StatusMark kind={st.kind} word={word.text} />
  {/if}
{/snippet}

<style>
  .h3 {
    margin: 20px 0 8px;
  }
</style>
