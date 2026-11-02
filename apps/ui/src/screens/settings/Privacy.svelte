<script lang="ts">
  import { tick } from "svelte";

  import { clearHistory } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { errorText } from "../../lib/errors";
  import { t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import Row from "./Row.svelte";
  import { type About, about, formatSize, openDataFolder } from "./data";

  let info = $state<About | null>(null);
  let failure = $state<unknown>(null);
  let confirming = $state(false);
  let clearing = $state(false);
  let confirmButton = $state<HTMLButtonElement>();
  let clearButton = $state<HTMLButtonElement>();

  $effect(() => {
    about().then(
      (answer) => (info = answer),
      (error) => (failure = error),
    );
  });

  async function ask(): Promise<void> {
    confirming = true;
    await tick();
    confirmButton?.focus();
  }

  async function cancel(): Promise<void> {
    confirming = false;
    await tick();
    clearButton?.focus();
  }

  async function clear(): Promise<void> {
    clearing = true;
    try {
      await clearHistory();
      toast.show(t("ui.settings.privacy.cleared"));
      info = await about();
      await cancel();
    } catch (error) {
      toast.show(errorText(error));
    } finally {
      clearing = false;
    }
  }

  async function open(): Promise<void> {
    try {
      await openDataFolder();
    } catch (error) {
      toast.show(errorText(error));
    }
  }
</script>

<Row id="privacy" wide>
  <p class="local">{t("ui.settings.privacy.local")}</p>
  <dl class="facts">
    <div>
      <dt>{t("ui.settings.privacy.version")}</dt>
      <dd>{info?.version ?? "…"}</dd>
    </div>
    <div>
      <dt>{t("ui.settings.privacy.used")}</dt>
      <dd>
        {#if info}
          {info.data_bytes === null ? t("ui.settings.privacy.nothing") : formatSize(info.data_bytes)}
        {:else}
          …
        {/if}
      </dd>
    </div>
  </dl>
  {#if failure !== null}
    <p class="failure" role="status">{errorText(failure)}</p>
  {/if}
  <div class="buttons">
    <button type="button" class="btn" disabled={info?.data_bytes === null} onclick={open}>
      {t("ui.settings.privacy.open")}
    </button>
    {#if !confirming}
      <button
        type="button"
        class="btn"
        disabled={!app.editable || info?.data_bytes === null}
        bind:this={clearButton}
        onclick={ask}
      >
        {t("ui.settings.privacy.clear")}
      </button>
    {/if}
  </div>
  {#if confirming}
    <div class="confirm">
      <p>{t("ui.settings.privacy.clear_ask")}</p>
      <div class="buttons">
        <button type="button" class="btn accent" disabled={clearing} bind:this={confirmButton} onclick={clear}>
          {t("ui.settings.privacy.clear_confirm")}
        </button>
        <button type="button" class="btn" disabled={clearing} onclick={cancel}>
          {t("ui.settings.privacy.clear_cancel")}
        </button>
      </div>
    </div>
  {/if}
</Row>

<style>
  p {
    margin: 0;
  }
  .local {
    max-width: 62ch;
    margin-bottom: 10px;
    color: var(--ink2);
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 28px;
    margin: 0 0 12px;
    color: var(--ink2);
    font: 400 14px/1.5 var(--f);
  }
  .facts div {
    display: flex;
    gap: 6px;
  }
  dd {
    margin: 0;
    color: var(--ink);
    font-weight: 600;
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .confirm {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--stroke2);
  }
  .confirm p {
    max-width: 62ch;
    margin-bottom: 12px;
    color: var(--ink);
  }
  .failure {
    margin-bottom: 12px;
    color: var(--ink2);
    font: 400 13px/1.4 var(--f);
  }
</style>
