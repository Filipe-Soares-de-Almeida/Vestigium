<script lang="ts">
  import Block from "../../components/Block.svelte";
  import Icon from "../../components/Icon.svelte";
  import { formatDuration } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { Hung } from "./live.svelte";

  let { hung, nowMs }: { hung: Hung[]; nowMs: number } = $props();
</script>

<Block title={t("ui.now.hung.title")}>
  {#if hung.length === 0}
    <p class="muted">{t("ui.now.hung.none")}</p>
  {:else}
    <ul>
      {#each hung as window (window.pid)}
        {@const duration = formatDuration(Math.max(0, nowMs - window.sinceMs))}
        <li>
          <Icon name="app" />
          <span>
            <b>{window.name}</b>
            <small>
              {t(window.exact ? "ui.now.hung.for" : "ui.now.hung.for_at_least", { duration })}
              {t("ui.now.hung.counted")}
            </small>
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</Block>

<style>
  p,
  ul {
    margin: 0;
  }
  ul {
    display: grid;
    gap: 10px;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 14px;
    align-items: center;
  }
  li > :global(svg) {
    color: var(--freeze);
  }
  b {
    display: block;
    font-weight: 400;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
</style>
