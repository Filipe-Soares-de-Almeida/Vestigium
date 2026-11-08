<script lang="ts">
  import Block from "../../components/Block.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import Steps from "../../components/Steps.svelte";
  import { app } from "../../lib/app.svelte";
  import { health } from "../../lib/health.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { at } from "../dumps/words";
  import { areaOf } from "../health/areas";
  import { findingView, valueText } from "../health/words";
  import DetailPage from "./DetailPage.svelte";
  import Facts from "./Facts.svelte";
  import Missing from "./Missing.svelte";

  let { id }: { id: number } = $props();

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const finding = $derived([...(health.dto?.open ?? []), ...(health.dto?.closed ?? [])].find((f) => f.id === id) ?? null);
  const view = $derived(finding ? findingView(finding, level) : null);
</script>

{#if health.dto === null}
  <DetailPage fallback={href.health()} title={t("ui.health.title")}><span></span></DetailPage>
{:else if finding && view}
  <DetailPage fallback={href.health(areaOf(finding.kind).id)} title={view.title} sub={t(`ui.health.area.${areaOf(finding.kind).id}`)}>
    <Facts
      rows={[
        { label: t("ui.detail.status"), value: statusText },
        { label: t("ui.detail.opened"), value: at(finding.opened_ms) },
        ...(finding.closed_ms !== null ? [{ label: t("ui.detail.closed"), value: at(finding.closed_ms) }] : []),
        { label: t("ui.detail.value_then"), value: finding.first_value ? valueText(finding.kind, finding.first_value) : "—" },
        { label: t("ui.detail.value_last"), value: view.value },
      ]}
    />
    <Block title={t("ui.detail.why")} first>
      <p class="text">{view.why}</p>
    </Block>
    {#if view.evidence.length}
      <Block title={t("ui.health.evidence")}>
        <ul class="ev">
          {#each view.evidence as line, i (i)}
            <li>{line.main}{#if line.sub}<small>{line.sub}</small>{/if}</li>
          {/each}
        </ul>
      </Block>
    {/if}
    {#if view.steps.length && finding.closed_ms === null}
      <Block title={t("ui.occurrence.todo.title")}>
        <Steps steps={view.steps} />
      </Block>
    {/if}
  </DetailPage>
{:else}
  <DetailPage fallback={href.health()} title={t("ui.health.title")}><Missing /></DetailPage>
{/if}

{#snippet statusText()}
  {#if finding}
    {#if finding.closed_ms !== null}
      <StatusMark kind="ok" word={t("ui.health.mark.Closed")} />
    {:else}
      <StatusMark kind={finding.concern === "Critical" ? "critical" : "attention"} />
    {/if}
  {/if}
{/snippet}

<style>
  .text {
    max-width: 70ch;
    margin: 0;
    line-height: 1.55;
  }
  .ev {
    display: grid;
    gap: 8px;
    max-width: 75ch;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .ev small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
</style>
