<script lang="ts">
  import Block from "../../components/Block.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { health } from "../../lib/health.svelte";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { AREAS, areaState } from "../health/areas";

  const touched = $derived(
    health.dto ? AREAS.map((a) => areaState(a, health.dto!)).filter((s) => s.open.length) : [],
  );
</script>

<Block title={t("ui.health.title")}>
  <p class="line">
    {#if !touched.length}
      {t("ui.home.health.none")}
    {:else if health.critical}
      {tn("ui.health.card.critical", health.critical)}
    {:else}
      {tn("ui.health.card.attention", health.attention)}
    {/if}
  </p>
  {#if touched.length}
    <RowList>
      {#each touched as s (s.area.id)}
        <RowCard href={href.health(s.area.id)} icon={s.area.icon} title={t(`ui.health.area.${s.area.id}`)}>
          {#snippet right()}<StatusMark kind={s.mark === "Critical" ? "critical" : "attention"} reveal />{/snippet}
        </RowCard>
      {/each}
    </RowList>
  {/if}
</Block>

<style>
  .line {
    margin: 0 0 12px;
    font-size: 14px;
  }
</style>
