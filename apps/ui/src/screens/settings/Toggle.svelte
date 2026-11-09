<script lang="ts">
  import { t } from "../../lib/i18n.svelte";

  // A Windows toggle switch, named by its setting row or by `label`.
  let {
    id,
    checked,
    disabled = false,
    label,
    onchange,
  }: { id: string; checked: boolean; disabled?: boolean; label?: string; onchange: (on: boolean) => void } = $props();
</script>

<button
  type="button"
  class="tg"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  aria-labelledby={label ? undefined : `${id}-name`}
  aria-describedby={label ? undefined : `${id}-hint`}
  data-focus={id}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <i aria-hidden="true"></i><span>{checked ? t("ui.settings.on") : t("ui.settings.off")}</span>
</button>

<style>
  .tg {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--ink);
    font: 400 14px/1.4 var(--f);
    cursor: pointer;
  }
  .tg:focus-visible {
    outline: none;
  }
  .tg:focus-visible i {
    outline: 2px solid var(--ink);
    outline-offset: 2px;
  }
  i {
    position: relative;
    flex: none;
    width: 40px;
    height: 20px;
    border: 1.5px solid var(--ink2);
    border-radius: 999px;
    transition:
      background 0.15s,
      border-color 0.15s;
  }
  i::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 3.5px;
    width: 10px;
    height: 10px;
    margin-top: -5px;
    border-radius: 50%;
    background: var(--ink2);
    transition: left 0.15s;
  }
  .tg:hover:enabled i {
    border-color: var(--ink);
  }
  .tg:hover:enabled i::after {
    background: var(--ink);
  }
  .tg[aria-checked="true"] i {
    background: var(--accent);
    border-color: var(--accent);
  }
  .tg[aria-checked="true"] i::after,
  .tg[aria-checked="true"]:hover:enabled i::after {
    left: 23.5px;
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
