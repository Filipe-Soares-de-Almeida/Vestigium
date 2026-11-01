<script lang="ts" module>
  // Category icons, 20 px line drawings in the navigation style.
  const CATEGORIES = {
    appearance:
      '<circle cx="10" cy="10" r="7.2" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M10 2.8a7.2 7.2 0 010 14.4z" fill="currentColor"/>',
    reports:
      '<rect x="4" y="2.5" width="12" height="15" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7 7h6M7 10h6M7 13h3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>',
    watching:
      '<path d="M1.8 10.5h3l2-5 3.4 9.5 2.1-4.5h5.9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>',
    history:
      '<ellipse cx="10" cy="5" rx="6.5" ry="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M3.5 5v10c0 1.4 2.9 2.5 6.5 2.5s6.5-1.1 6.5-2.5V5M3.5 10c0 1.4 2.9 2.5 6.5 2.5s6.5-1.1 6.5-2.5" fill="none" stroke="currentColor" stroke-width="1.5"/>',
    language:
      '<circle cx="10" cy="10" r="7.2" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M2.8 10h14.4M10 2.8c2.2 2.4 2.2 12 0 14.4M10 2.8c-2.2 2.4-2.2 12 0 14.4" fill="none" stroke="currentColor" stroke-width="1.5"/>',
    privacy:
      '<path d="M10 2.5l6 2.5v4.6c0 3.6-2.5 6.6-6 7.9-3.5-1.3-6-4.3-6-7.9V5z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M7.5 10l1.8 1.8L12.8 8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>',
  } as const;

  type Category = keyof typeof CATEGORIES;
</script>

<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href, router } from "../lib/router.svelte";
  import Appearance from "./settings/Appearance.svelte";
  import History from "./settings/History.svelte";
  import Language from "./settings/Language.svelte";
  import Privacy from "./settings/Privacy.svelte";
  import Reports from "./settings/Reports.svelte";
  import Watching from "./settings/Watching.svelte";

  const current = $derived.by((): Category => {
    const route = router.current;
    const wanted = route.screen === "settings" ? route.category : null;
    return wanted && wanted in CATEGORIES ? (wanted as Category) : "appearance";
  });
  const settings = $derived(app.settings?.settings ?? null);
</script>

<h1>{t("ui.settings.title")}</h1>
{#if settings === null}
  <p class="lede">{t("ui.settings.unavailable")}</p>
{:else}
  <p class="lede">{app.editable ? t("ui.settings.lede") : t("ui.settings.read_only")}</p>
  <div class="cats">
    <nav class="catlist" aria-label={t("ui.settings.categories")}>
      {#each Object.entries(CATEGORIES) as [category, icon] (category)}
        <a
          class="cat"
          href={href.settings(category)}
          aria-current={category === current ? "page" : undefined}
        >
          <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true" focusable="false">
            {@html icon}
          </svg>
          {t(`ui.settings.category.${category}`)}
        </a>
      {/each}
    </nav>
    <section class="panel" aria-labelledby="settings-panel">
      <h2 id="settings-panel">{t(`ui.settings.category.${current}`)}</h2>
      <div class="setlist">
        {#if current === "appearance"}
          <Appearance {settings} />
        {:else if current === "reports"}
          <Reports {settings} />
        {:else if current === "watching"}
          <Watching />
        {:else if current === "history"}
          <History {settings} />
        {:else if current === "language"}
          <Language {settings} />
        {:else}
          <Privacy />
        {/if}
      </div>
    </section>
  </div>
{/if}

<style>
  .cats {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    gap: 22px;
    align-items: start;
  }
  .catlist {
    position: sticky;
    top: 0;
    display: grid;
    gap: 2px;
  }
  .cat {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 40px;
    padding: 0 12px;
    border-radius: 6px;
    color: var(--ink);
    font: 400 14px/1.4 var(--f);
    text-decoration: none;
  }
  .cat svg {
    flex: none;
    color: var(--ink2);
  }
  .cat:hover {
    background: var(--stroke2);
  }
  .cat[aria-current="page"] {
    background: var(--stroke2);
    font-weight: 600;
  }
  .cat[aria-current="page"]::before {
    content: "";
    position: absolute;
    left: 0;
    top: 11px;
    bottom: 11px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }
  .panel h2 {
    margin: 6px 0 12px;
  }
  .setlist {
    display: grid;
    gap: 4px;
  }
  .setlist :global(select) {
    height: 32px;
    min-width: 0;
    padding: 0 10px;
    border: 1px solid var(--stroke);
    border-radius: 8px;
    background: var(--card);
    color: var(--ink);
    font: 400 14px/1.4 var(--f);
    accent-color: var(--accent);
    cursor: pointer;
  }
  .setlist :global(select:hover:enabled) {
    background: var(--card2);
  }
  .setlist :global(select:disabled) {
    color: var(--ink3);
    cursor: default;
  }
  @container content (max-width: 720px) {
    .cats {
      grid-template-columns: minmax(0, 1fr);
    }
    .catlist {
      position: static;
      display: flex;
      flex-wrap: wrap;
      gap: 4px;
    }
  }
  @media (forced-colors: active) {
    .cat[aria-current="page"]::before {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
