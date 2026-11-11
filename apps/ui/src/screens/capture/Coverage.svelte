<script lang="ts">
  import StatusMark from "../../components/StatusMark.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { CoverLine } from "./words";

  let { rows }: { rows: CoverLine[] } = $props();
</script>

<table class="tbl cov">
  <caption class="sr">{t("ui.captures.cover.caption")}</caption>
  <thead>
    <tr>
      <th scope="col">{t("ui.captures.cover.col.source")}</th>
      <th scope="col">{t("ui.captures.cover.col.measured")}</th>
      <th scope="col">{t("ui.captures.cover.col.kept")}</th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row (row.part)}
      <tr data-part={row.part}>
        <td>
          <span class="src">
            <StatusMark kind={row.state === "full" ? "ok" : "attention"} word={row.minutes} />
            <span>
              {row.name}
              {#if row.tool}<small>{row.tool}</small>{/if}
            </span>
          </span>
        </td>
        <td>
          {row.minutes}
          {#if row.why}<small>{row.why}</small>{/if}
          {#if row.reason}<small><code>{row.reason}</code></small>{/if}
          <div class="bar" class:none={row.state === "none"} role="img" aria-label={row.minutes}>
            <i style:width="{row.fraction * 100}%"></i>
          </div>
        </td>
        <td>{row.kept}</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .cov {
    width: 100%;
  }
  .cov td {
    white-space: normal;
    vertical-align: top;
  }
  .cov small {
    display: block;
    margin-top: 2px;
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .cov code {
    font: 12.5px var(--mono);
  }
  .src {
    display: flex;
    align-items: flex-start;
    gap: 2px;
  }
  .src :global(.word) {
    display: none;
  }
  .src :global(.status) {
    padding-top: 2px;
  }
  .src > span:last-child {
    padding-left: 8px;
  }
  .bar {
    position: relative;
    width: 140px;
    height: 8px;
    margin-top: 8px;
    border-radius: 4px;
    background: var(--hover);
    overflow: hidden;
  }
  .bar i {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    border-radius: 4px;
    background: var(--calm);
  }
  .bar.none {
    background: repeating-linear-gradient(135deg, var(--ink2) 0 1.2px, transparent 1.2px 5px);
    opacity: 0.5;
  }
  @media (forced-colors: active) {
    .bar i {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
