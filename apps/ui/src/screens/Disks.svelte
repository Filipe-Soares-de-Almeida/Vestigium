<script lang="ts">
  import { onMount } from "svelte";

  import Block from "../components/Block.svelte";
  import ErrorView from "../components/ErrorView.svelte";
  import Loading from "../components/Loading.svelte";
  import RowCard from "../components/RowCard.svelte";
  import RowList from "../components/RowList.svelte";
  import StatusMark from "../components/StatusMark.svelte";
  import { app } from "../lib/app.svelte";
  import { health } from "../lib/health.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import type { DiskDto, FindingDto } from "../lib/types";
  import { findingTitle } from "./health/words";
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
</script>

<header class="head">
  <h1>{t("ui.group.disks")}</h1>
  <p class="sub">{t("ui.group.disks.sub")}</p>
</header>

{#if dto === null && hwtests.error !== null}
  <ErrorView error={hwtests.error} heading="h2" onretry={() => void hwtests.load()} />
{:else if dto === null}
  <Loading />
{:else if dto.disks.length === 0}
  <p class="muted">{t("ui.disks.none")}</p>
{:else}
  <div class="cols">
    {#each dto.disks as d, i (d.number)}
      {@const state = diskState(dto, d, hwtests.details[d.number] ?? null, running)}
      {@const st = statusOf(state.mark)}
      {@const alerts = alertsOf(d)}
      {@const attrs = watched(d)}
      <Block title="{nameOf(d)} · {volumes(d)}" first={i === 0}>
        <p class="state"><StatusMark kind={st.kind} word={st.word} /><span>{state.line}</span></p>

        <h3>{t("ui.disks.smart")}</h3>
        {#if attrs.length}
          <dl class="smart">
            {#each attrs as a (a.id)}
              <div><dt>{attrName(a.id)}</dt><dd>{a.raw}</dd></div>
            {/each}
          </dl>
        {:else}
          <p class="muted">{t("ui.disks.smart.none")}</p>
        {/if}

        <h3>{t("ui.disks.alerts")}</h3>
        {#if alerts.length}
          <RowList>
            {#each alerts as f (f.id)}
              <RowCard href={to(href.health("disks"))} icon="health" title={findingTitle(f, level)}>
                {#snippet right()}<StatusMark kind={f.concern === "Critical" ? "critical" : "attention"} reveal />{/snippet}
              </RowCard>
            {/each}
          </RowList>
        {:else}
          <p class="muted">{t("ui.disks.alerts.none")}</p>
        {/if}
        <p class="more"><a href={to(href.tests(String(d.number)))}>{t("ui.disks.open_tests")}</a></p>
      </Block>
    {/each}
  </div>
{/if}

<style>
  .head {
    margin-bottom: 4px;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0 56px;
  }
  @container content (min-width: 1000px) {
    .cols {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      align-items: start;
    }
  }
  .state {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 14px;
    margin: 0 0 14px;
    font-size: 14px;
  }
  h3 {
    margin: 16px 0 8px;
    font: 600 14px/1.3 var(--f);
  }
  .muted {
    margin: 0;
    color: var(--ink2);
    font-size: 14px;
  }
  .smart {
    display: grid;
    gap: 4px;
    margin: 0;
  }
  .smart div {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    padding: 6px 12px;
    border-radius: 6px;
    background: var(--row);
    font-size: 14px;
  }
  .smart dt {
    color: var(--ink2);
  }
  .smart dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .more {
    margin: 14px 0 0;
    font-size: 14px;
  }
  .more a {
    font-weight: 600;
  }
</style>
