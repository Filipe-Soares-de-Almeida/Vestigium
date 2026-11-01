<script lang="ts">
  import Icon from "../../components/Icon.svelte";
  import { formatDuration } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { Hung } from "./live.svelte";

  let { hung, nowMs }: { hung: Hung[]; nowMs: number } = $props();
</script>

<section class="card" aria-labelledby="hung-title">
  <h2 id="hung-title">{t("ui.now.hung.title")}</h2>
  {#if hung.length === 0}
    <p class="muted">{t("ui.now.hung.none")}</p>
  {:else}
    <ul>
      {#each hung as window (window.pid)}
        {@const duration = formatDuration(Math.max(0, nowMs - window.sinceMs))}
        <li>
          <span class="icochip"><Icon name="app" /></span>
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
</section>

<style>
  h2 {
    font: 600 15px/1.3 var(--f);
    margin: 0 0 8px;
  }
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
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }
  .icochip {
    background: var(--freeze-t);
    color: var(--freeze);
  }
  b {
    display: block;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    color: var(--ink2);
    font-size: 13px;
  }
</style>
