<script lang="ts">
  import Segmented from "../../components/Segmented.svelte";
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

<Row id="mode" title={t("ui.settings.appearance.mode")} hint={t("ui.settings.appearance.mode.hint")}>
  <Segmented
    label={t("ui.settings.appearance.mode")}
    options={MODES.map((mode) => ({ value: mode, label: t(`ui.settings.appearance.mode.${mode}`) }))}
    value={settings.theme}
    disabled={!app.editable}
    onchange={(theme) => save({ theme })}
  />
</Row>
<Row id="color" title={t("ui.settings.appearance.color")} hint={t("ui.settings.appearance.color.hint")} wide>
  <div
    class="swatches"
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
        aria-checked={theme === settings.color_theme}
        tabindex={theme === settings.color_theme ? 0 : -1}
        data-focus="color:{theme}"
        disabled={!app.editable}
        onclick={() => theme !== settings.color_theme && save({ color_theme: theme })}
      >
        <span class="sw" data-swatch={SWATCH[theme]} aria-hidden="true">
          <i class="calm"></i><i class="busy"></i><i class="press"></i><i class="accent"></i>
        </span>
        {theme === "Red" ? t("ui.settings.appearance.color.default", { name }) : name}
      </button>
    {/each}
  </div>
</Row>

<style>
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px 6px 6px;
    border: 1px solid var(--stroke2);
    border-radius: 10px;
    background: var(--card2);
    color: var(--ink);
    font: 600 13px/1.4 var(--f);
    cursor: pointer;
  }
  button:hover:enabled {
    border-color: var(--stroke);
  }
  button[aria-checked="true"] {
    border-color: transparent;
    box-shadow: 0 0 0 2px var(--accent);
  }
  button:disabled {
    cursor: default;
    color: var(--ink3);
  }
  .sw {
    display: flex;
    flex: none;
    align-items: flex-end;
    gap: 2px;
    width: 44px;
    height: 28px;
    padding: 5px;
    border: 1px solid var(--stroke2);
    border-radius: 7px;
    background: var(--base);
  }
  .sw i {
    flex: 1;
    height: 70%;
    border-radius: 2px;
  }
  .sw .calm {
    background: var(--calm);
  }
  .sw .busy {
    background: var(--busy);
  }
  .sw .press {
    background: var(--press);
  }
  .sw .accent {
    height: 100%;
    background: var(--accent);
  }
  button:disabled .sw {
    opacity: 0.6;
  }
  @media (forced-colors: active) {
    button[aria-checked="true"] {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }
    .sw {
      forced-color-adjust: none;
    }
  }
</style>
