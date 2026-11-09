<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../../components/Block.svelte";
  import ErrorView from "../../components/ErrorView.svelte";
  import Loading from "../../components/Loading.svelte";
  import { occurrence } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import type { OccurrenceDto } from "../../lib/types";
  import { at } from "../dumps/words";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  let { occurrenceId, recordId }: { occurrenceId: number; recordId: number } = $props();

  /** `undefined` while it loads, `null` when the history no longer has it. */
  let dto = $state<OccurrenceDto | null | undefined>(undefined);
  let failure = $state<unknown>(null);

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const event = $derived(dto?.window.events.find((e) => e.record_id === recordId) ?? null);

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
  <DetailPage fallback={href.occurrence(occurrenceId)} title={t("ui.detail.event.title_plain")}>
    <ErrorView error={failure} heading="h2" onretry={load} />
  </DetailPage>
{:else if dto === undefined}
  <DetailPage fallback={href.occurrence(occurrenceId)} title={t("ui.detail.event.title_plain")}><Loading /></DetailPage>
{:else if event}
  <DetailPage
    fallback={href.occurrence(occurrenceId)}
    title={t(`ui.occurrence.event.${event.meaning}`)}
    sub={t("ui.detail.event.sub", { provider: event.provider, id: event.id })}
  >
    <Facts
      rows={[
        { label: t("ui.detail.event.time"), value: at(event.ts_ms) },
        { label: t("ui.detail.event.channel"), value: event.channel },
        { label: t("ui.detail.event.provider"), value: event.provider },
        { label: t("ui.detail.event.id"), value: String(event.id) },
        { label: t("ui.detail.event.record"), value: String(event.record_id) },
        { label: t("ui.detail.event.level"), value: String(event.level) },
      ]}
    />
    {#if event.data_xml && level !== "Simple"}
      <Block title={t("ui.detail.event.data")}>
        <pre>{event.data_xml}</pre>
      </Block>
    {/if}
  </DetailPage>
{:else}
  <DetailPage fallback={href.occurrence(occurrenceId)} title={t("ui.detail.event.title_plain")}><Missing /></DetailPage>
{/if}

<style>
  pre {
    max-height: 360px;
    margin: 0;
    overflow: auto;
    color: var(--ink2);
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    font-size: 12.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
