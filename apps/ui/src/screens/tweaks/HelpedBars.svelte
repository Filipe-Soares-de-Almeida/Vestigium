<script lang="ts">
  import { t, tn } from "../../lib/i18n.svelte";
  import type { Helped } from "../../lib/types";
  import { helpedWhy, helpedWord, lacksData, monitoredHours, rateText, ratePerHour } from "./words";

  let { helped, metric }: { helped: Helped; metric: string } = $props();

  const sides = $derived([
    { key: "before", label: t("ui.tweaks.helped.before"), events: helped.before, ms: helped.before_ms },
    { key: "after", label: t("ui.tweaks.helped.after"), events: helped.after, ms: helped.after_ms },
  ]);
  const top = $derived(Math.max(...sides.map((s) => ratePerHour(s.events, s.ms)), Number.MIN_VALUE));
  const lacking = $derived(lacksData(helped));
  const why = $derived(helpedWhy(helped));
  const least = $derived(helped.min_hours * 3_600_000);
  const label = $derived(
    tn("ui.tweaks.helped.label", helped.days, {
      metric,
      before: helped.before,
      before_hours: monitoredHours(helped.before_ms),
      after: helped.after,
      after_hours: monitoredHours(helped.after_ms),
    }),
  );
</script>

<div class="helped" class:lacking role="img" aria-label={label}>
  {#each sides as side (side.key)}
    <div class="side">
      <span>{side.label}</span>
      <i class={side.key} style:width="{Math.max((ratePerHour(side.events, side.ms) / top) * 100, 3)}%"></i>
      <span class="val">
        <b>{lacking && helped.verdict === "NotEnoughEvents" ? tn("ui.tweaks.helped.events", side.events) : rateText(side.events, side.ms)}</b>
        <small>
          {t("ui.tweaks.helped.of_hours", { count: side.events, hours: monitoredHours(side.ms) })}
          {#if side.ms < least}<em>{t("ui.tweaks.helped.short", { hours: monitoredHours(side.ms) })}</em>{/if}
        </small>
      </span>
    </div>
  {/each}
  <p class="verdict">
    <b>{helpedWord(helped)}</b>
    <small>· {tn("ui.tweaks.helped.span", helped.days, { metric })}</small>
  </p>
  {#if why}<p class="why">{why}</p>{/if}
</div>

<style>
  .helped {
    display: grid;
    gap: 6px;
    max-width: 460px;
    margin-top: 10px;
    font-size: 13px;
  }
  .side {
    display: grid;
    grid-template-columns: 56px minmax(0, 1fr) 140px;
    gap: 10px;
    align-items: center;
  }
  .side > span {
    color: var(--ink2);
  }
  i {
    display: block;
    height: 8px;
    border-radius: 4px;
    background: var(--ink3);
  }
  i.after {
    background: var(--accent);
  }
  .lacking i {
    background: transparent;
    border: 1.5px dashed var(--ink3);
  }
  .val {
    text-align: right;
    line-height: 1.25;
  }
  .val b {
    display: block;
    font-weight: 600;
  }
  .lacking .val b {
    color: var(--ink2);
    font-weight: 400;
  }
  small,
  .why {
    color: var(--ink2);
  }
  .val small {
    font-size: 12px;
  }
  em {
    margin-left: 6px;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--hover);
    font-style: normal;
    white-space: nowrap;
  }
  .verdict,
  .why {
    margin: 0;
  }
  .verdict b {
    font-weight: 600;
    color: var(--ink);
  }
  .lacking .verdict b {
    color: var(--ink2);
    font-weight: 400;
  }
</style>
