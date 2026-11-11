<script lang="ts">
  import { tick } from "svelte";

  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { historyReset } from "../lib/historyReset.svelte";
  import { t } from "../lib/i18n.svelte";
  import { toast } from "../lib/toast.svelte";
  import { updates } from "../lib/update.svelte";
  import { openDataFolder } from "../screens/settings/data";
  import StatusMark from "./StatusMark.svelte";

  // Tells once, on the first opening after the watcher set a damaged history aside, what was kept.
  let dialog = $state<HTMLDialogElement>();
  let first = $state<HTMLButtonElement>();

  const reset = $derived(app.monitor?.history_reset ?? null);
  // The update questions go first; this one waits until none is open.
  const owed = $derived(historyReset.owed(reset) && updates.dialog === null ? reset : null);
  const rows = $derived(
    reset === null
      ? []
      : [
          { key: "tweaks", kept: reset.tweaks },
          { key: "settings", kept: reset.settings },
          { key: "machine", kept: reset.machine },
        ],
  );

  $effect(() => {
    if (!dialog) return;
    if (owed && !dialog.open) {
      dialog.showModal();
      void tick().then(() => first?.focus());
    } else if (!owed && dialog.open) {
      dialog.close();
    }
  });

  async function openFolder(): Promise<void> {
    try {
      await openDataFolder();
    } catch (error) {
      toast.show(errorText(error));
    }
  }

  function close(): void {
    if (reset) historyReset.see(reset);
  }
</script>

<dialog
  class="reset"
  bind:this={dialog}
  aria-labelledby="reset-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  {#if reset}
    <div class="in">
      <h2 id="reset-title">{t("ui.history.reset.title")}</h2>
      <p>{t("ui.history.reset.body", { file: reset.file })}</p>
      <ul>
        {#each rows as row (row.key)}
          <li>
            <span>{t(`ui.history.reset.item.${row.key}`)}</span>
            <StatusMark
              kind={row.kept ? "ok" : "attention"}
              word={row.kept ? t("ui.history.reset.kept") : t("ui.history.reset.not_kept")}
            />
          </li>
        {/each}
      </ul>
      {#if !reset.tweaks}<p>{t("ui.history.reset.lost")}</p>{/if}
      <p>{t("ui.history.reset.never_deleted")}</p>
    </div>
    <div class="acts">
      <button type="button" class="btn" onclick={openFolder}>{t("ui.history.reset.open_folder")}</button>
      <button type="button" class="btn accent" bind:this={first} onclick={close}>{t("ui.history.reset.got_it")}</button>
    </div>
  {/if}
</dialog>

<style>
  .reset {
    width: min(520px, calc(100vw - 48px));
    max-height: min(620px, calc(100vh - 80px));
    padding: 0;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .reset[open] {
    display: flex;
    flex-direction: column;
  }
  .reset::backdrop {
    background: #0009;
  }
  .in {
    display: grid;
    gap: 10px;
    padding: 20px 22px 16px;
    overflow: auto;
  }
  h2 {
    margin: 0;
    font: 600 20px/1.3 var(--fd);
  }
  p {
    margin: 0;
    color: var(--ink2);
    font-size: 15px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 6px 12px;
    border-radius: 8px;
    background: var(--row);
    font-size: 15px;
  }
  li :global(.status) {
    font-size: 12px;
  }
  li :global(.cut) {
    stroke: var(--card);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
    padding: 14px 22px;
    border-top: 1px solid var(--stroke2);
    border-radius: 0 0 12px 12px;
    background: var(--card2);
  }
  @media (forced-colors: active) {
    .reset {
      border-color: CanvasText;
    }
  }
</style>
