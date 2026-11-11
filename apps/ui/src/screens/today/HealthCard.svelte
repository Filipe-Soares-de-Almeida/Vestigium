<script lang="ts">
  import Block from "../../components/Block.svelte";
  import RowCard from "../../components/RowCard.svelte";
  import RowList from "../../components/RowList.svelte";
  import StatusMark from "../../components/StatusMark.svelte";
  import { app } from "../../lib/app.svelte";
  import { health } from "../../lib/health.svelte";
  import { t, tn } from "../../lib/i18n.svelte";
  import { href } from "../../lib/router.svelte";
  import { areaOf } from "../health/areas";
  import { findingTitle } from "../health/words";

  const level = $derived(app.settings?.settings.detail_level ?? "Simple");
  /** Open findings, critical first, each with the area it opens. */
  const open = $derived(
    (health.dto?.open ?? [])
      .slice()
      .sort((a, b) => Number(b.concern === "Critical") - Number(a.concern === "Critical") || a.opened_ms - b.opened_ms)
      .map((f) => ({ f, area: areaOf(f.kind) })),
  );
  const line = $derived.by(() => {
    if (health.dto === null) return health.error !== null ? t("ui.home.health.failed") : t("ui.common.loading");
    const { critical, attention } = health;
    if (!critical && !attention) return t("ui.home.health.none");
    if (critical && attention) {
      return t("ui.health.card.mixed", {
        critical: tn("ui.health.card.n_critical", critical),
        attention: tn("ui.health.card.n_attention", attention),
      });
    }
    return critical ? tn("ui.health.card.critical", critical) : tn("ui.health.card.attention", attention);
  });
</script>

<Block title={t("ui.health.title")}>
  <p class="p">{line}</p>
  {#if open.length}
    <RowList>
      {#each open as { f, area } (`${f.kind}:${JSON.stringify(f.subject)}`)}
        <RowCard href={href.health(area.id)} title={findingTitle(f, level)} sub={t(`ui.health.area.${area.id}`)}>
          {#snippet right()}<StatusMark kind={f.concern === "Critical" ? "critical" : "attention"} reveal />{/snippet}
        </RowCard>
      {/each}
    </RowList>
  {/if}
  <p class="more"><a href={href.health()}>{t("ui.today.health.open")}</a></p>
</Block>
