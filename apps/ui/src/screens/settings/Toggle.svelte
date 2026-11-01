<script lang="ts">
  import { t } from "../../lib/i18n.svelte";

  // A Windows toggle switch named by its setting row.
  let {
    id,
    checked,
    disabled = false,
    onchange,
  }: { id: string; checked: boolean; disabled?: boolean; onchange: (on: boolean) => void } = $props();
</script>

<button
  type="button"
  class="tg"
  role="switch"
  aria-checked={checked}
  aria-labelledby="{id}-name"
  aria-describedby="{id}-hint"
  data-focus={id}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span>{checked ? t("ui.settings.on") : t("ui.settings.off")}</span><i aria-hidden="true"></i>
</button>

<style>
  .tg {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 4px;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--ink2);
    font: 400 13px/1.4 var(--f);
    cursor: pointer;
  }
  i {
    position: relative;
    flex: none;
    width: 40px;
    height: 20px;
    border: 1px solid var(--ink2);
    border-radius: 999px;
    transition:
      background 0.15s,
      border-color 0.15s;
  }
  i::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--ink2);
    transition: left 0.15s;
  }
  .tg:hover:enabled i {
    border-color: var(--ink);
  }
  .tg[aria-checked="true"] i {
    background: var(--accent);
    border-color: var(--accent);
  }
  .tg[aria-checked="true"] i::after {
    left: 23px;
    background: var(--on-accent);
  }
  .tg:disabled {
    cursor: default;
    color: var(--ink3);
  }
  .tg:disabled i {
    opacity: 0.5;
  }
  @media (forced-colors: active) {
    i {
      forced-color-adjust: none;
      border-color: CanvasText;
      background: Canvas;
    }
    i::after {
      background: CanvasText;
    }
    .tg[aria-checked="true"] i {
      border-color: Highlight;
      background: Highlight;
    }
    .tg[aria-checked="true"] i::after {
      background: HighlightText;
    }
    .tg:disabled i {
      border-color: GrayText;
    }
  }
</style>
