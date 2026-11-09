<script lang="ts">
  import { app } from "../../lib/app.svelte";
  import { errorText } from "../../lib/errors";
  import { radioGroup } from "../../lib/focus";
  import { t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import type { ColorTheme, Settings, ThemePref } from "../../lib/types";
  import Row from "./Row.svelte";
  import { saveSettings } from "./data";

  let { settings }: { settings: Settings } = $props();

  const MODES: ThemePref[] = ["System", "Light", "Dark"];
  const THEMES: ColorTheme[] = ["Red", "Graphite", "Slate", "Indigo"];
  /** The `data-swatch` that draws each theme with its own tokens. */
  const SWATCH: Record<ColorTheme, string> = {
    Red: "red",
    Graphite: "graphite",
    Slate: "slate",
    Indigo: "indigo",
  };

  async function save(change: Partial<Settings>): Promise<void> {
    try {
      await saveSettings(change);
    } catch (error) {
      toast.show(errorText(error));
    }
  }
</script>

<Row id="mode" title={t("ui.settings.appearance.mode")}>
  <div class="rds v" role="radiogroup" aria-labelledby="mode-name">
    {#each MODES as mode (mode)}
      <label class="rd">
        <input
          type="radio"
          name="theme-mode"
          checked={settings.theme === mode}
          disabled={!app.editable}
          onchange={() => save({ theme: mode })}
        />
        <span class="rd-c" aria-hidden="true"></span>
        <span class="rd-l">{t(`ui.settings.appearance.mode.${mode}`)}</span>
      </label>
    {/each}
  </div>
</Row>
<Row id="color" title={t("ui.settings.appearance.color")} hint={t("ui.settings.appearance.color.hint")} wide>
  <div
    class="sw"
    role="radiogroup"
    aria-labelledby="color-name"
    aria-describedby="color-hint"
    aria-disabled={!app.editable || undefined}
    use:radioGroup
  >
    {#each THEMES as theme (theme)}
      {@const name = t(`ui.common.color_theme.${theme}`)}
      <button
        type="button"
        role="radio"
        class="sw-b"
        aria-checked={theme === settings.color_theme}
        tabindex={theme === settings.color_theme ? 0 : -1}
        data-focus="color:{theme}"
        disabled={!app.editable}
        onclick={() => theme !== settings.color_theme && save({ color_theme: theme })}
      >
        <span class="sw-c" data-swatch={SWATCH[theme]} aria-hidden="true"><i></i></span>
        <span class="sw-n">{theme === "Red" ? t("ui.settings.appearance.color.default", { name }) : name}</span>
      </button>
    {/each}
  </div>
</Row>

<style>
  .sw {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  .sw-b {
    display: grid;
    justify-items: center;
    align-content: start;
    gap: 10px;
    width: 132px;
    padding: 10px 6px;
    border: 0;
    border-radius: 10px;
    background: none;
    color: var(--ink);
    font: 400 13px/1.4 var(--f);
    cursor: pointer;
  }
  .sw-b:hover:enabled {
    background: var(--hover);
  }
  .sw-b:disabled {
    cursor: default;
    color: var(--ink3);
  }
  .sw-c {
    position: relative;
    display: block;
    width: 64px;
    height: 64px;
    overflow: hidden;
    border-radius: 12px;
    background: var(--accent);
  }
  .sw-c i {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 16px;
    background: linear-gradient(90deg, var(--calm) 0 33.4%, var(--busy) 33.4% 66.7%, var(--press) 66.7%);
  }
  .sw-b[aria-checked="true"] .sw-c {
    box-shadow:
      0 0 0 2px var(--main),
      0 0 0 4px var(--ink);
  }
  .sw-b[aria-checked="true"] .sw-n {
    font-weight: 600;
  }
  .sw-b:disabled .sw-c {
    opacity: 0.6;
  }
  @media (forced-colors: active) {
    .sw-b[aria-checked="true"] {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }
    .sw-c,
    .sw-c i {
      forced-color-adjust: none;
    }
  }
</style>
