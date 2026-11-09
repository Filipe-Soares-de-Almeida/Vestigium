<script lang="ts">
  import { tick } from "svelte";

  import { clearHistory } from "../../lib/api";
  import { app } from "../../lib/app.svelte";
  import { errorText } from "../../lib/errors";
  import { t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import Icon from "../../components/Icon.svelte";
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

<p class="lede">{t("ui.settings.privacy.local")}</p>
<Row id="privacy-data" title={t("ui.settings.privacy.data")} wide>
  <dl class="kv">
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
    <p class="note" role="status">{errorText(failure)}</p>
  {/if}
  <div class="acts">
    <button type="button" class="btn" disabled={info?.data_bytes === null} onclick={open}>
      <Icon name="outside" />{t("ui.settings.privacy.open")}
    </button>
  </div>
</Row>
<Row id="privacy-clear" title={t("ui.settings.privacy.clear")} wide>
  {#if confirming}
    <div class="confirm" role="group" aria-label={t("ui.settings.privacy.clear")}>
      <span>{t("ui.settings.privacy.clear_ask")}</span>
      <button type="button" class="btn" disabled={clearing} bind:this={confirmButton} onclick={clear}>
        <Icon name="trash" />{t("ui.settings.privacy.clear_confirm")}
      </button>
      <button type="button" class="btn" disabled={clearing} onclick={cancel}>
        {t("ui.settings.privacy.clear_cancel")}
      </button>
    </div>
  {:else}
    <p class="p">{t("ui.settings.privacy.clear_hint")}</p>
    <div class="acts">
      <button
        type="button"
        class="btn"
        disabled={!app.editable || info?.data_bytes === null}
        bind:this={clearButton}
        onclick={ask}
      >
        <Icon name="trash" />{t("ui.settings.privacy.clear_more")}
      </button>
    </div>
  {/if}
</Row>

<style>
  .lede {
    margin: 16px 0 0;
  }
  .kv {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 8px 32px;
    margin: 0;
    font-size: 14px;
  }
  .kv div {
    display: contents;
  }
  .kv dt {
    color: var(--ink2);
  }
  .kv dd {
    margin: 0;
  }
  .confirm {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 14px;
    max-width: 840px;
    padding: 12px 14px;
    border-radius: 8px;
    background: var(--hover);
    font-size: 14px;
  }
  .confirm > span {
    flex: 1 1 280px;
  }
</style>
