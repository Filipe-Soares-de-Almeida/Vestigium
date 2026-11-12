<script lang="ts">
  import { tick } from "svelte";

  import { FILES, monitorHint, otherVersion, openPage, type Page } from "../lib/about";
  import { aboutDialog } from "../lib/aboutDialog.svelte";
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/errors";
  import { lang, t } from "../lib/i18n.svelte";
  import { releaseDay } from "../lib/notes";
  import { toast } from "../lib/toast.svelte";
  import { updates } from "../lib/update.svelte";
  import AboutCredits from "./AboutCredits.svelte";
  import Mark from "./Mark.svelte";
  import StatusMark from "./StatusMark.svelte";

  // The About dialog: the version line, what runs on this PC, and the credits.
  type Tab = "versions" | "credits";
  const TABS: readonly Tab[] = ["versions", "credits"];

  let dialog = $state<HTMLDialogElement>();
  let tab = $state<Tab>("versions");
  const tabs = $state<Record<Tab, HTMLButtonElement | undefined>>({ versions: undefined, credits: undefined });

  const facts = $derived(aboutDialog.facts);
  const version = $derived(facts?.version ?? updates.version);
  const day = $derived(releaseDay(facts?.released ?? null));
  const hint = $derived(facts ? monitorHint(facts) : null);
  const settings = $derived(app.settings?.settings ?? null);
  const language = $derived(t(lang() === "pt-BR" ? "ui.settings.language.PtBr" : "ui.settings.language.En"));

  $effect(() => {
    if (!dialog) return;
    if (aboutDialog.open && !dialog.open) {
      tab = "versions";
      dialog.showModal();
      void tick().then(() => tabs.versions?.focus());
    } else if (!aboutDialog.open && dialog.open) {
      dialog.close();
    }
  });

  /** Arrows move between the tabs and select the one they reach; Home and End jump to the ends. */
  function onTabKey(event: KeyboardEvent): void {
    const at = TABS.indexOf(tab);
    const next =
      event.key === "ArrowRight"
        ? TABS[(at + 1) % TABS.length]
        : event.key === "ArrowLeft"
          ? TABS[(at - 1 + TABS.length) % TABS.length]
          : event.key === "Home"
            ? TABS[0]
            : event.key === "End"
              ? TABS[TABS.length - 1]
              : null;
    if (!next) return;
    event.preventDefault();
    tab = next;
    tabs[next]?.focus();
  }

  async function open(page: Page): Promise<void> {
    try {
      await openPage(page);
    } catch {
      toast.show(t("ui.about.open_failed"));
    }
  }
</script>

<dialog
  class="about"
  bind:this={dialog}
  aria-labelledby="about-title"
  aria-describedby="about-line"
  oncancel={(event) => {
    event.preventDefault();
    aboutDialog.close();
  }}
>
  <header class="head">
    <Mark size={44} />
    <div>
      <h2 id="about-title">{t("ui.shell.title")}</h2>
      <p id="about-line">
        {day ? t("ui.about.released", { version, date: day }) : t("ui.update.version", { version })}
      </p>
      <p>{t("ui.about.tagline")}</p>
    </div>
  </header>

  <div class="tabs" role="tablist" aria-label={t("ui.about.title")} tabindex="-1" onkeydown={onTabKey}>
    {#each TABS as key (key)}
      <button
        type="button"
        class="tab"
        role="tab"
        id="about-tab-{key}"
        aria-selected={tab === key}
        aria-controls="about-panel"
        tabindex={tab === key ? 0 : -1}
        bind:this={tabs[key]}
        onclick={() => (tab = key)}
      >
        {t(`ui.about.tab.${key}`)}
      </button>
    {/each}
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="body" role="tabpanel" id="about-panel" aria-labelledby="about-tab-{tab}" tabindex="0">
    {#if tab === "versions"}
      {#if facts}
        {@const monitor = facts.monitor}
        {@const runs = t(monitor.run === "stopped" ? "ui.about.monitor.stopped" : "ui.about.monitor.running")}
        {@const monitorOther = monitor.run === "other" ? null : otherVersion(monitor.version, facts.version)}
        {@const helperOther = otherVersion(facts.helper, facts.version)}
        <section class="grp" aria-labelledby="about-app">
          <h3 id="about-app">{t("ui.about.this_app")}</h3>
          <dl class="kv">
            <div>
              <dt>{t("ui.about.window")}</dt>
              <dd>{FILES.window}</dd>
            </div>
            <div class="monitor">
              <dt>{t("ui.about.monitor")}</dt>
              <dd>
                {#if monitor.run === "other"}
                  <span class="vm"><StatusMark kind="attention" size={14} reveal />{t("ui.about.monitor.other")}</span>
                  <small>
                    {monitor.protocol === null
                      ? FILES.monitor
                      : `${t("ui.about.monitor.speaks", { protocol: monitor.protocol, expected: facts.protocol })} · ${FILES.monitor}`}
                  </small>
                {:else if monitorOther}
                  <span class="vm"><StatusMark kind="attention" size={14} reveal />{monitorOther}</span>
                  <small>{runs} · {FILES.monitor}</small>
                {:else}
                  {runs} <small>{FILES.monitor}</small>
                {/if}
              </dd>
            </div>
            <div class="helper">
              <dt>{t("ui.about.helper")}</dt>
              <dd>
                {#if facts.helper === null}
                  {t("ui.about.helper.missing")} <small>{FILES.helper}</small>
                {:else if helperOther}
                  {helperOther} <small>{t("ui.about.helper.on_demand")} · {FILES.helper}</small>
                {:else}
                  {t("ui.about.helper.on_demand")} <small>{FILES.helper}</small>
                {/if}
              </dd>
            </div>
            <div>
              <dt>{t("ui.about.schema")}</dt>
              <dd>{facts.schema}</dd>
            </div>
            <div>
              <dt>{t("ui.about.protocol")}</dt>
              <dd>{facts.protocol}</dd>
            </div>
          </dl>
          {#if hint}
            <p class="hint"><StatusMark kind="attention" size={16} reveal /><span>{hint}</span></p>
          {/if}
        </section>
        <section class="grp" aria-labelledby="about-pc">
          <h3 id="about-pc">{t("ui.about.this_pc")}</h3>
          <dl class="kv">
            {#if facts.windows}
              <div>
                <dt>{t("ui.about.windows")}</dt>
                <dd>{facts.windows.release ? `${facts.windows.edition}, ${facts.windows.release}` : facts.windows.edition}</dd>
              </div>
              <div>
                <dt>{t("ui.about.build")}</dt>
                <dd>{facts.windows.build}</dd>
              </div>
            {/if}
            {#if facts.webview}
              <div>
                <dt>{t("ui.about.webview")}</dt>
                <dd>{facts.webview}</dd>
              </div>
            {/if}
            <div>
              <dt>{t("ui.about.language")}</dt>
              <dd>
                {language}
                {#if settings?.language === "System"}<small>{t("ui.about.as_windows")}</small>{/if}
              </dd>
            </div>
            {#if settings}
              <div>
                <dt>{t("ui.common.detail.label")}</dt>
                <dd>{t(`ui.common.detail.${settings.detail_level}`)}</dd>
              </div>
            {/if}
          </dl>
        </section>
      {:else if aboutDialog.failure !== null}
        <p class="note" role="status">{errorText(aboutDialog.failure)}</p>
      {:else}
        <p class="note">{t("ui.common.loading")}</p>
      {/if}
    {:else}
      <AboutCredits {open} />
    {/if}
  </div>

  <footer class="foot">
    <span class="sp"></span>
    <button type="button" class="btn" onclick={() => aboutDialog.close()}>{t("ui.about.close")}</button>
  </footer>
</dialog>

<style>
  /* One height for both tabs, so switching between them leaves the dialog where it is. */
  .about {
    width: min(540px, calc(100vw - 48px));
    height: min(660px, calc(100vh - 56px));
    max-height: none;
    padding: 0;
    border: 1px solid var(--stroke);
    border-radius: 12px;
    background: var(--card);
    color: var(--ink);
    box-shadow: var(--shadow-float);
  }
  .about[open] {
    display: flex;
    flex-direction: column;
  }
  .about::backdrop {
    background: #0007;
  }
  .about :global(.cut) {
    stroke: var(--card);
  }
  .head {
    display: flex;
    flex: none;
    align-items: center;
    gap: 16px;
    padding: 20px 22px 0;
  }
  h2 {
    margin: 0;
    font: 600 20px/1.3 var(--fd);
  }
  .head p {
    margin: 2px 0 0;
    color: var(--ink2);
    font-size: 13px;
    line-height: 1.45;
  }
  .tabs {
    display: flex;
    flex: none;
    gap: 4px;
    margin-top: 14px;
    padding: 0 16px;
    border-bottom: 1px solid var(--stroke2);
  }
  .tab {
    position: relative;
    height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: 6px 6px 0 0;
    background: none;
    color: var(--ink2);
    font-size: 14px;
    cursor: pointer;
  }
  .tab:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .tab[aria-selected="true"] {
    color: var(--ink);
    font-weight: 600;
  }
  .tab[aria-selected="true"]::after {
    content: "";
    position: absolute;
    bottom: -1px;
    left: 50%;
    width: 18px;
    height: 3px;
    margin-left: -9px;
    border-radius: 2px;
    background: var(--accent);
  }
  .tab:focus-visible {
    outline-offset: -2px;
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 16px 22px 18px;
    overflow: auto;
  }
  .body:focus-visible {
    outline-offset: -2px;
  }
  .body :global(.grp + .grp) {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--rule);
  }
  .body :global(.grp h3) {
    margin: 0 0 10px;
    font: 600 14px/1.35 var(--f);
  }
  .body :global(.kv) {
    display: grid;
    grid-template-columns: minmax(184px, max-content) minmax(0, 1fr);
    gap: 6px 24px;
    margin: 0;
    font-size: 13.5px;
    line-height: 1.45;
  }
  .body :global(.kv > div) {
    display: contents;
  }
  .body :global(.kv dt) {
    color: var(--ink2);
  }
  .body :global(.kv dd) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    min-width: 0;
    margin: 0;
  }
  .body :global(.kv dd small) {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .vm {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .hint {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin: 14px 0 0;
    padding: 10px 14px;
    border-radius: 8px;
    background: var(--busy-t);
    font-size: 13.5px;
    line-height: 1.45;
  }
  .hint :global(.status) {
    margin-top: 1px;
  }
  .hint :global(.cut) {
    stroke: var(--busy-t);
  }
  .note {
    margin: 0;
  }
  .foot {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 14px 22px;
    border-top: 1px solid var(--stroke2);
    border-radius: 0 0 12px 12px;
    background: var(--card2);
  }
  .sp {
    flex: 1;
  }
  @media (forced-colors: active) {
    .about {
      border-color: CanvasText;
    }
    .tab[aria-selected="true"]::after {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
