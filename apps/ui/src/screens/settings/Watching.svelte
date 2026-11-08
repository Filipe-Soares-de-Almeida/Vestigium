<script lang="ts">
  import { setAutostart, setPaused, setSensitive } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { errorText } from "../../lib/errors";
  import { keepFocus } from "../../lib/focus";
  import { formatTime } from "../../lib/format";
  import { t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import Row from "./Row.svelte";
  import Toggle from "./Toggle.svelte";

  const autostart = $derived(app.settings?.autostart ?? null);
  const sensitiveUntil = $derived(
    app.link?.link === "running" ? app.link.sensitive_until_ms : null,
  );
  let busy = $state(false);

  /** Runs one change at a time and toasts what failed; the controls stay focusable meanwhile. */
  async function run(change: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await change();
    } catch (error) {
      toast.show(errorText(error));
    } finally {
      busy = false;
    }
  }

  const changeAutostart = (on: boolean) =>
    run(async () => {
      await setAutostart(on);
      await app.loadSettings();
      toast.show(t(on ? "ui.settings.watching.autostart.on" : "ui.settings.watching.autostart.off"));
    });

  const changeSensitive = (on: boolean) =>
    run(async () => {
      await setSensitive(on);
      await app.refresh();
      toast.show(
        sensitiveUntil !== null
          ? t("ui.settings.watching.sensitive.on", { time: formatTime(sensitiveUntil) })
          : t("ui.settings.watching.sensitive.off"),
      );
    });

  /** Pausing swaps the button for Resume; focus moves to the new one. */
  function changePaused(paused: boolean): void {
    const main = document.getElementById("main");
    const change = () =>
      run(async () => {
        if (!paused) return app.resume();
        await setPaused(true);
        await app.refresh();
        toast.show(t("ui.settings.watching.pause.done"));
      });
    if (main) void keepFocus(main, change);
    else void change();
  }
</script>

<Row id="autostart" title={t("ui.settings.watching.startup")} label={t("ui.settings.watching.autostart")} hint={t("ui.settings.watching.autostart.hint")}>
  {#if autostart === null}
    <span class="unknown">{t("ui.settings.watching.autostart.unknown")}</span>
  {:else}
    <Toggle
      id="autostart"
      checked={autostart}
      disabled={!app.editable}
      onchange={changeAutostart}
    />
  {/if}
</Row>
<Row
  id="sensitive"
  title={t("ui.settings.watching.sensitivity")}
  label={t("ui.settings.watching.sensitive")}
  hint={sensitiveUntil !== null
    ? t("ui.settings.watching.sensitive.until", { time: formatTime(sensitiveUntil) })
    : t("ui.settings.watching.sensitive.hint")}
>
  <Toggle
    id="sensitive"
    checked={sensitiveUntil !== null}
    disabled={!app.running}
    onchange={changeSensitive}
  />
</Row>
<Row
  id="pause"
  title={t("ui.settings.watching.pause")}
  hint={app.paused ? t("ui.settings.watching.pause.paused") : t("ui.settings.watching.pause.hint")}
>
  {#if app.paused}
    <button
      type="button"
      class="btn accent"
      data-focus="pause"
      aria-describedby="pause-hint"
      onclick={() => changePaused(false)}>{t("ui.shell.paused.resume")}</button
    >
  {:else}
    <button
      type="button"
      class="btn"
      data-focus="pause"
      aria-describedby="pause-hint"
      disabled={!app.running}
      onclick={() => changePaused(true)}>{t("ui.settings.watching.pause.action")}</button
    >
  {/if}
</Row>

<style>
  .unknown {
    color: var(--ink2);
    font: 400 13px/1.4 var(--f);
  }
</style>
