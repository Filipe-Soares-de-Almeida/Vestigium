<script lang="ts">
  import { onMount } from "svelte";

  import BackLink from "../components/BackLink.svelte";
  import Block from "../components/Block.svelte";
  import HelperDialogs from "../components/HelperDialogs.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import { occurrence } from "../lib/api";
  import { captures } from "../lib/capture.svelte";
  import { errorText } from "../lib/errors";
  import { formatTime } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { href } from "../lib/router.svelte";
  import History from "./captures/History.svelte";
  import Notice from "./captures/Notice.svelte";
  import Start from "./captures/Start.svelte";

  let { forOccurrence }: { forOccurrence: number | null } = $props();

  /** The occurrence this page was opened from, so the start card can say what it is for. */
  let from = $state<{ id: number; context: string } | null>(null);

  relatedLinks(() => [
    { label: t("ui.related.events_today"), href: href.events() },
    ...(from ? [{ label: t("ui.related.back_report"), href: href.occurrence(from.id) }] : []),
  ]);

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
  <BackLink href={href.occurrence(from.id)} label={t("ui.captures.back")} />
{/if}
<PageHead title={t("ui.captures.title")} />
<p class="lede">{t("ui.captures.lede")}</p>

<HelperDialogs
  job="capture"
  onconfirm={() => void captures.cancel()}
  onretry={() => void captures.begin(captures.lastMinutes)}
/>

<div class="cols2">
  <Block title={t("ui.captures.start.title")}>
    {#if captures.asking}
      <Notice id="ask" title={t("ui.captures.asking.title")} body={t("ui.captures.asking.body")} />
    {:else if captures.problem === "declined"}
      <Notice id="declined" title={t("ui.captures.declined.title")} body={t("ui.captures.declined.body")}>
        <button type="button" class="btn accent" onclick={() => captures.dismiss()}>{t("ui.common.retry")}</button>
      </Notice>
    {:else if captures.problem === "interrupted"}
      <Notice id="interrupted" title={t("ui.captures.interrupted.title")} body={t("ui.captures.interrupted.body")}>
        <button type="button" class="btn accent" onclick={() => captures.dismiss()}>{t("ui.common.retry")}</button>
      </Notice>
    {:else if captures.problem === "failed"}
      <Notice id="failed" title={t("ui.captures.failed.title")} body={errorText(captures.problemError)}>
        <button type="button" class="btn accent" onclick={() => captures.dismiss()}>{t("ui.common.retry")}</button>
      </Notice>
    {:else}
      <Start context={from?.context ?? null} />
    {/if}
  </Block>

  {#if captures.entries === null}
    <Block title={t("ui.captures.history.title")}><Loading /></Block>
  {:else}
    <History entries={captures.entries} fresh={captures.finished} />
  {/if}
</div>
