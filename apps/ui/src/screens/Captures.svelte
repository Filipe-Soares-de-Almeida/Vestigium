<script lang="ts">
  import { onMount } from "svelte";

  import Icon from "../components/Icon.svelte";
  import Loading from "../components/Loading.svelte";
  import { occurrence } from "../lib/api";
  import { captures } from "../lib/capture.svelte";
  import { errorText } from "../lib/errors";
  import { formatTime } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { rowOf } from "./capture/words";
  import History from "./captures/History.svelte";
  import Notice from "./captures/Notice.svelte";
  import Start from "./captures/Start.svelte";
  import Waiting from "./captures/Waiting.svelte";

  let { forOccurrence }: { forOccurrence: number | null } = $props();

  /** The occurrence this page was opened from, so the start card can say what it is for. */
  let from = $state<{ id: number; context: string } | null>(null);

  const finished = $derived(captures.entries?.find((e) => e.summary.id === captures.finished) ?? null);

  onMount(() => {
    if (forOccurrence === null) return;
    const id = forOccurrence;
    occurrence(id)
      .then((found) => {
        if (!found) return;
        const time = formatTime(found.start_ms);
        from = { id, context: t(found.kind === "Stutter" ? "ui.captures.context.Stutter" : "ui.captures.context.default", { time }) };
      })
      .catch(() => {});
  });
</script>

{#if from}
  <a class="back" href={href.occurrence(from.id)}><Icon name="back" />{t("ui.captures.back")}</a>
{/if}
<h1>{t("ui.captures.title")}</h1>
<p class="lede">{t("ui.captures.lede")}</p>

<div class="state">
  {#if captures.run}
    <Waiting run={captures.run} />
  {:else if captures.asking}
    <Notice id="ask" icon="shield" title={t("ui.captures.asking.title")} body={t("ui.captures.asking.body")} />
  {:else if captures.problem === "declined"}
    <Notice id="declined" icon="shield" title={t("ui.captures.declined.title")} body={t("ui.captures.declined.body")}>
      <button type="button" class="btn accent" onclick={() => captures.dismiss()}>{t("ui.common.retry")}</button>
    </Notice>
  {:else if captures.problem === "interrupted"}
    <Notice id="interrupted" icon="info" title={t("ui.captures.interrupted.title")} body={t("ui.captures.interrupted.body")}>
      <button type="button" class="btn accent" onclick={() => captures.dismiss()}>{t("ui.common.retry")}</button>
    </Notice>
  {:else if captures.problem === "failed"}
    <Notice id="failed" icon="info" title={t("ui.captures.failed.title")} body={errorText(captures.problemError)}>
      <button type="button" class="btn accent" onclick={() => captures.dismiss()}>{t("ui.common.retry")}</button>
    </Notice>
  {:else if finished}
    {@const row = rowOf(finished)}
    <Notice id="finished" icon="wave" title={t("ui.captures.finished.title")} body={`${row.title}. ${row.sub}`}>
      <a class="btn accent" href={href.capture(finished.summary.id)}>{t("ui.captures.finished.open")}</a>
      <button type="button" class="btn" onclick={() => captures.dismiss()}>{t("ui.captures.finished.again")}</button>
    </Notice>
  {:else}
    <Start context={from?.context ?? null} />
  {/if}
</div>

<div class="history">
  {#if captures.entries === null}
    <Loading />
  {:else}
    <History entries={captures.entries} fresh={captures.finished} />
  {/if}
</div>

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
    border-radius: 6px;
    color: var(--ink2);
    font: 600 14px/1.5 var(--f);
    text-decoration: none;
  }
  .back:hover {
    color: var(--ink);
  }
  .history {
    margin-top: 14px;
  }
</style>
