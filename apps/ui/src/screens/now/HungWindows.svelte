<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { formatDuration } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { Hung } from "./live.svelte";

  let { hung, nowMs, notice }: { hung: Hung[]; nowMs: number; /** Set while the monitor is silent: the list is old and cannot say "none". */ notice?: string } = $props();
</script>

<Block title={t("ui.now.hung.title")} {notice}>
  {#if hung.length === 0 && notice !== undefined}
    <!-- Nothing to list: the notice says it cannot tell. -->
  {:else if hung.length === 0}
    <p class="empty">{t("ui.now.hung.none")}</p>
  {:else}
    <ul class="ev-l">
      {#each hung as window (window.pid)}
        {@const duration = formatDuration(Math.max(0, nowMs - window.sinceMs))}
        <li>
          {t(window.exact ? "ui.now.hung.line" : "ui.now.hung.line_at_least", { name: window.name, duration })}
        </li>
      {/each}
    </ul>
  {/if}
</Block>
