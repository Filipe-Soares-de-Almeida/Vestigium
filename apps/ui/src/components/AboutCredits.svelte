<script lang="ts">
  import { tick } from "svelte";

  import { AUTHOR, type Page, allRows, filterRows, licenseText, mainRows } from "../lib/about";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import TextField from "./TextField.svelte";

  // The Credits tab: who made Vestigium, its license, the open-source components and the links.
  let { open }: { open: (page: Page) => void } = $props();

  let all = $state(false);
  let query = $state("");

  const main = $derived(mainRows());
  const every = $derived(allRows());
  const rows = $derived(all ? filterRows(every, query) : main);
  const narrowed = $derived(all && query.trim() !== "");

  async function pick(next: boolean): Promise<void> {
    all = next;
    query = "";
    if (!next) return;
    await tick();
    document.getElementById("about-filter")?.focus();
  }
</script>

<section class="grp">
  <dl class="kv">
    <div>
      <dt>{t("ui.about.made_by")}</dt>
      <dd>
        <button type="button" class="lnk" onclick={() => open("author")}>{AUTHOR}<Icon name="outside" /></button>
      </dd>
    </div>
    <div>
      <dt>{t("ui.about.license")}</dt>
      <dd>
        <button type="button" class="lnk" onclick={() => open("license")}>{licenseText("MIT")}<Icon name="outside" /></button>
      </dd>
    </div>
  </dl>
</section>

<section class="grp" aria-labelledby="about-components">
  <h3 id="about-components">{t("ui.about.components")}</h3>
  <div class="lic">
    <div class="bar">
      <span class="seg" role="group" aria-label={t("ui.about.components.which")}>
        <button type="button" aria-pressed={!all} onclick={() => pick(false)}>
          {t("ui.about.components.main", { count: main.length })}
        </button>
        <button type="button" aria-pressed={all} onclick={() => pick(true)}>
          {t("ui.about.components.all", { count: every.length })}
        </button>
      </span>
      {#if all}
        <span class="find">
          <TextField
            id="about-filter"
            label={t("ui.about.components.filter_label")}
            placeholder={t("ui.about.components.filter")}
            bind:value={query}
          />
        </span>
      {/if}
      <span class="sp"></span>
      <span class="count" role="status">
        {narrowed ? t("ui.about.components.shown", { shown: rows.length, total: every.length }) : ""}
      </span>
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="scroll" role="region" aria-label={t("ui.about.components")} tabindex="0">
      <table class="tbl">
        <thead>
          <tr>
            <th class="fill">{t("ui.about.components.name")}</th>
            <th>{t("ui.about.components.version")}</th>
            <th>{t("ui.about.components.license")}</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.key)}
            <tr>
              <td class="fill" title={row.caption ? `${row.name} · ${row.caption}` : row.name}>
                {row.name}{#if row.caption}<small>{row.caption}</small>{/if}
              </td>
              <td class="dim">{row.version}</td>
              <td class="dim">{row.license}</td>
            </tr>
          {:else}
            <tr><td class="none" colspan="3">{t("ui.about.components.none")}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="foot">{t(all ? "ui.about.components.all_note" : "ui.about.components.main_note")}</p>
  </div>
</section>

<section class="grp">
  <div class="links">
    <button type="button" class="lnk" onclick={() => open("repository")}>{t("ui.about.link.repository")}<Icon name="outside" /></button>
    <button type="button" class="lnk" onclick={() => open("wiki")}>{t("ui.about.link.wiki")}<Icon name="outside" /></button>
    <button type="button" class="lnk" onclick={() => open("privacy")}>{t("ui.about.link.privacy")}<Icon name="outside" /></button>
  </div>
</section>

<style>
  .lnk {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--accent);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .lnk:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .lnk :global(svg) {
    flex: none;
    width: 13px;
    height: 13px;
  }
  .lic {
    border: 1px solid var(--stroke2);
    border-radius: 10px;
    background: var(--card);
    overflow: hidden;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--stroke2);
  }
  .seg {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--field);
  }
  .seg button {
    height: 28px;
    padding: 0 11px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--ink2);
    font-size: 13px;
    white-space: nowrap;
    cursor: pointer;
  }
  .seg button:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .seg button[aria-pressed="true"] {
    background: var(--row-hover);
    color: var(--ink);
    font-weight: 600;
  }
  .seg button:focus-visible {
    outline-offset: 0;
  }
  .find {
    width: 150px;
  }
  .find :global(.field) {
    height: 32px;
    font-size: 13.5px;
  }
  .sp {
    flex: 1;
  }
  .count {
    color: var(--ink2);
    font-size: 12.5px;
  }
  .scroll {
    max-height: 170px;
    overflow: auto;
    scrollbar-gutter: stable;
  }
  .scroll:focus-visible {
    outline-offset: -2px;
  }
  .tbl {
    font-size: 13px;
  }
  .tbl th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--card);
  }
  .tbl td {
    padding: 7px 12px;
  }
  .tbl small {
    margin-left: 8px;
    color: var(--ink2);
    font-size: 12px;
  }
  .dim {
    color: var(--ink2);
  }
  .none {
    padding: 14px 12px;
    color: var(--ink2);
  }
  .foot {
    margin: 0;
    padding: 8px 12px;
    border-top: 1px solid var(--stroke2);
    color: var(--ink2);
    font-size: 12px;
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 20px;
    font-size: 13.5px;
  }
</style>
