<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";

  /** Marks an action that asks Windows for administrator permission. */
  let { text }: { text?: string } = $props();
</script>

<span class="pill">
  <Icon name="admin" />
  <span>{text ?? t("ui.admin.required")}</span>
</span>

<style>
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 12px 4px 9px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 60%, transparent);
    color: var(--ink);
    font: 600 12.5px/1.35 var(--f);
    white-space: nowrap;
  }
  .pill :global(svg) {
    color: var(--accent);
  }
</style>
