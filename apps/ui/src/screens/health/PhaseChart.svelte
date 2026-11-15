<script lang="ts">
  import Spark from "../../components/Spark.svelte";
  import ZoomBar from "../../components/ZoomBar.svelte";
  import { t, tn } from "../../lib/i18n.svelte";
  import type { BootDto } from "../../lib/types";
  import type { Zoom } from "../../lib/zoom.svelte";
  import { ceilingOf, formatSeconds, type Phase, phaseOf, series, stamp } from "./startup";

  /** One phase across the startups shown, from zero, with the zoom the other time charts have. */
  let { boots, phase }: { boots: BootDto[]; phase: Phase } = $props();

  let zoom = $state<Zoom>();
  let plot = $state<HTMLElement>();

  const values = $derived(series(boots, phase.id));
  const ceiling = $derived(ceilingOf(boots, phase.id));
  const limit = $derived(phaseOf(phase.id).limit);
  const at = (i: number) => stamp(boots[Math.min(boots.length - 1, Math.max(0, Math.round(i)))].kernel_start_ms);
  const range = $derived(zoom ? t("ui.zoom.range", { from: at(zoom.from), to: at(zoom.from + zoom.span) }) : "");
</script>

<div class="head">
  <h3>{tn("ui.health.startup.last", boots.length)}</h3>
  {#if zoom}<ZoomBar {zoom} {plot} {range} />{/if}
</div>
<Spark
  {values}
  label={t(`ui.health.startup.phase.${phase.id}`)}
  from={at(0)}
  to={at(boots.length - 1)}
  when={at}
  format={(seconds) => formatSeconds(seconds * 1000)}
  missing={t("ui.health.startup.value.unmeasured")}
  floor={0}
  {ceiling}
  ceilingLabel="{ceiling} s"
  reference={limit === null ? undefined : { value: limit, label: `${limit} s` }}
  bind:zoom
  bind:plot
/>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 16px;
    margin: 18px 0 6px;
  }
  h3 {
    margin: 0;
  }
</style>
