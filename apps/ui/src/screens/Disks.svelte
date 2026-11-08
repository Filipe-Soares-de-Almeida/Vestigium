<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import PageHead from "../components/PageHead.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { t } from "../lib/i18n.svelte";
  import { relatedLinks } from "../lib/related.svelte";
  import { here, href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import type { DiskDto, FindingDto } from "../lib/types";
  import { findingTitle, findingView } from "./health/words";
  import { formatBytes } from "./occurrence/format";
  import { statusOf } from "./tests/status";
  import { attrName, diskState, WATCHED } from "./tests/words";

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  const dto = $derived(hwtests.dto);
  const running = $derived(hwtests.running !== null);

  onMount(() => {
    if (hwtests.dto === null) void hwtests.load();
  });

  const nameOf = (d: DiskDto) => t(`ui.tests.disk.name.${d.media}`, { size: d.bytes ? formatBytes(d.bytes) : "" }).trim();
  const volumes = (d: DiskDto) => d.volumes.map((v) => v.replace(/\\$/, "")).join(", ");

  /** The open Health alerts about a disk or one of its volumes. */
  function alertsOf(d: DiskDto): FindingDto[] {
    return (health.dto?.open ?? []).filter((f) => {
      const s = f.subject;
      if (typeof s !== "object") return false;
      if ("Disk" in s) return s.Disk === d.name || s.Disk.split(" ")[0] === String(d.number);
      return d.volumes.includes(s.Volume);
    });
  }

  /** The SMART attributes that warn and have been read for this disk. */
  const watched = (d: DiskDto) =>
    (hwtests.details[d.number]?.health?.attributes ?? []).filter((a) => WATCHED.includes(a.id));

  const to = (path: string) => `${path}${path.includes("?") ? "&" : "?"}g=disks`;
  const subOf = (d: DiskDto) => t("ui.tests.disk.sub", { n: d.number, bus: t(`ui.tests.bus.${d.bus}`), volumes: volumes(d) });

  relatedLinks(() => [
    { label: t("ui.related.disk_health"), href: href.health("disks") },
    { label: t("ui.group.tests"), href: href.tests() },
  ]);
</script>

<PageHead title={t("ui.group.disks")} sub={t("ui.group.disks.sub")} />

{#if dto === null && hwtests.error !== null}
  <ErrorView error={hwtests.error} heading="h2" onretry={() => void hwtests.load()} />
{:else if dto === null}
  <Loading />
{:else if dto.disks.length === 0}
  <p class="empty">{t("ui.disks.none")}</p>
{:else}
  <div class="cols2">
    {#each dto.disks as d (d.number)}
      {@const state = diskState(dto, d, hwtests.details[d.number] ?? null, running)}
      {@const st = statusOf(state.mark)}
      {@const alerts = alertsOf(d)}
      {@const attrs = watched(d)}
      <Block title="{nameOf(d)} · {volumes(d)}" icon={d.media === "Hdd" ? "hdd" : "ssd"}>
        <p class="state"><StatusMark kind={st.kind} word={st.word} /><span>{subOf(d)}</span></p>
        <p class="p">{state.line}</p>
        {#if attrs.length}
          <RowList>
            {#each attrs as a (a.id)}
              <RowCard href={href.detail("smart", [d.number, a.id], { from: here(), label: t("ui.group.disks") })} title={attrName(a.id)}>
                {#snippet right()}
                  <b class="value">{a.raw}</b>
                  {#if a.raw > 0}<StatusMark kind="attention" reveal />{/if}
                {/snippet}
              </RowCard>
            {/each}
          </RowList>
        {:else}
          <p class="empty">{t("ui.disks.smart.none")}</p>
        {/if}

        <h3 class="h3">{t("ui.disks.alerts")}</h3>
        {#if alerts.length}
          <RowList>
            {#each alerts as f (f.id)}
              {@const view = findingView(f, level)}
              <RowCard href={to(href.health("disks"))} title={findingTitle(f, level)} sub="{t('ui.health.ruler.now')} {view.value}">
                {#snippet right()}<StatusMark kind={f.concern === "Critical" ? "critical" : "attention"} reveal />{/snippet}
              </RowCard>
            {/each}
          </RowList>
        {:else}
          <p class="empty">{t("ui.disks.alerts.none")}</p>
        {/if}
        <p class="more"><a href={to(href.tests(String(d.number)))}>{t("ui.disks.open_tests")}</a></p>
      </Block>
    {/each}
  </div>
{/if}

<style>
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    margin: 0 0 10px;
    color: var(--ink2);
    font-size: 13px;
  }
  .value {
    color: var(--ink);
    font-size: 14px;
    font-weight: 600;
  }
  .h3 {
    margin: 20px 0 8px;
  }
</style>
