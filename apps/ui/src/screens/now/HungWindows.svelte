<script lang="ts">
  import Block from "../../components/Block.svelte";
  import { formatDuration } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import type { Hung } from "./live.svelte";

  let { hung, nowMs }: { hung: Hung[]; nowMs: number } = $props();
</script>

<Block title={t("ui.now.hung.title")}>
  {#if hung.length === 0}
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
