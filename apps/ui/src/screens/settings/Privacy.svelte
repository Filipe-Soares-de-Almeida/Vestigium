<script lang="ts">
  import { errorText } from "../../lib/errors";
  import { t } from "../../lib/i18n.svelte";
  import { toast } from "../../lib/toast.svelte";
  import Row from "./Row.svelte";
  import { type About, about, formatSize, openDataFolder } from "./data";

  let info = $state<About | null>(null);
  let failure = $state<unknown>(null);

  $effect(() => {
    about().then(
      (answer) => (info = answer),
      (error) => (failure = error),
    );
  });

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
  <button type="button" class="btn" disabled={info?.data_bytes === null} onclick={open}>
    {t("ui.settings.privacy.open")}
  </button>
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
  .failure {
    margin-bottom: 12px;
    color: var(--ink2);
    font: 400 13px/1.4 var(--f);
  }
</style>
