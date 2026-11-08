<script lang="ts">
  import Finder from "../components/Finder.svelte";
  import Icon from "../components/Icon.svelte";
  import Mark from "../components/Mark.svelte";
  import StatusMark, { type StatusKind } from "../components/StatusMark.svelte";
  import { health } from "../lib/health.svelte";
  import { buildGroups, entryHref } from "../lib/groups";
  import { t, tn } from "../lib/i18n.svelte";
  import { href } from "../lib/router.svelte";
  import { shellStatus } from "../lib/status";
  import { tweaks } from "../lib/tweaks.svelte";

  const status = $derived(shellStatus());
  const tiles = $derived(buildGroups().filter((g) => !g.hidden));

  const open = $derived(health.dto?.open.length ?? 0);
  const critical = $derived(health.critical);
  const healthLine = $derived(
    open === 0
      ? t("ui.home.health.none")
      : critical
        ? `${tn("ui.shell.nav.health.open", open)} · ${tn("ui.shell.nav.health.critical", critical)}`
        : tn("ui.shell.nav.health.open", open),
  );
  const healthKind: StatusKind = $derived(critical ? "critical" : open ? "attention" : "ok");
  const tweakLine = $derived(
    tweaks.recommended ? tn("ui.shell.nav.tweaks.signals", tweaks.recommended) : t("ui.home.tweaks.none"),
  );
  const tweakKind: StatusKind = $derived(tweaks.recommended ? "attention" : "ok");
</script>

<div class="home">
  <header class="band">
    <div class="head">
      <div class="id">
        <span class="mark"><Mark /></span>
        <div class="who">
          <h1>{t("ui.shell.title")}</h1>
          <p class="state"><i class={status.tone} aria-hidden="true"></i>{t(status.key)}</p>
        </div>
      </div>
      <ul class="quick" aria-label={t("ui.home.situation")}>
        <li>
          <a class="card" href={href.health()} aria-label="{t('ui.group.health')}: {healthLine}">
            <span class="ic"><Icon name="health" /><span class="dot"><StatusMark kind={healthKind} size={14} reveal /></span></span>
            <b>{t("ui.group.health")}</b>
            <small>{healthLine}</small>
          </a>
        </li>
        <li>
          <a class="card" href={href.activity()}>
            <span class="ic"><Icon name="today" /></span>
            <b>{t("ui.group.activity")}</b>
            <small>{t("ui.home.activity")}</small>
          </a>
        </li>
        <li>
          <a class="card" href={href.tweaks()} aria-label="{t('ui.group.tweaks')}: {tweakLine}">
            <span class="ic"><Icon name="tweaks" /><span class="dot"><StatusMark kind={tweakKind} size={14} reveal /></span></span>
            <b>{t("ui.group.tweaks")}</b>
            <small>{tweakLine}</small>
          </a>
        </li>
        <li>
          <a class="card settings" href={href.settings()}>
            <span class="ic"><Icon name="settings" /></span>
            <b>{t("ui.group.settings")}</b>
            <small>{t("ui.home.settings")}</small>
          </a>
        </li>
      </ul>
    </div>
  </header>
  <div class="find"><Finder id="home-find" big autofocus /></div>
  <div class="grid">
    <ul class="tiles">
      {#each tiles as group (group.key)}
        <li>
          <a class="tile" href={entryHref(group)}>
            <span class="tile-ic"><Icon name={group.icon} /></span>
            <span class="text"><b>{group.name}</b><small>{group.sub}</small></span>
          </a>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .home {
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }
  .band {
    padding-bottom: 30px;
    background: var(--card2);
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 24px 40px;
    max-width: 1440px;
    margin: 0 auto;
    padding: 18px 48px 0;
  }
  .id {
    display: flex;
    align-items: center;
    gap: 24px;
    min-width: 0;
  }
  .mark {
    display: grid;
    flex: none;
    place-items: center;
    width: 72px;
    height: 72px;
  }
  .mark :global(svg) {
    width: 72px;
    height: 72px;
  }
  h1 {
    margin: 0 0 4px;
    font: 600 28px/1.15 var(--fd);
    letter-spacing: -0.01em;
  }
  .state {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 4px;
    color: var(--ink2);
    font-size: 13.5px;
  }
  .state i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .state i.ok {
    background: var(--good);
  }
  .state i.paused {
    background: var(--press);
  }
  .state i.off {
    border: 1.5px solid var(--ink3);
  }
  .quick {
    display: flex;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .card,
  .tile {
    border-radius: 8px;
    background: var(--row);
    box-shadow: inset 0 0 0 1px transparent;
    color: var(--ink);
    text-decoration: none;
  }
  .card:hover,
  .tile:hover {
    background: var(--row-hover);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink3) 55%, transparent);
  }
  .card:active,
  .tile:active {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .card:focus-visible,
  .tile:focus-visible {
    outline: 2px solid var(--ink);
    outline-offset: -2px;
  }
  .card {
    display: grid;
    gap: 2px;
    align-content: start;
    width: 204px;
    padding: 12px;
  }
  .settings {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .settings:hover {
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }
  .settings .ic {
    color: var(--accent);
  }
  .ic {
    position: relative;
    width: 34px;
    height: 34px;
    margin-bottom: 10px;
  }
  .ic > :global(svg),
  .tile-ic :global(svg) {
    color: inherit;
  }
  .ic > :global(svg) {
    width: 32px;
    height: 32px;
  }
  .ic :global(svg *),
  .tile-ic :global(svg *) {
    vector-effect: non-scaling-stroke;
  }
  .dot {
    position: absolute;
    right: -7px;
    bottom: -5px;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--card2), var(--ink) 4%);
  }
  .dot :global(.word) {
    display: none;
  }
  .card b,
  .tile b {
    display: block;
    font-size: 15px;
    font-weight: 400;
    line-height: 1.35;
  }
  .card small,
  .tile small {
    display: block;
    color: var(--ink2);
    font-size: 12.5px;
    line-height: 1.4;
  }
  .find {
    position: relative;
    z-index: 3;
    display: flex;
    justify-content: center;
    padding: 32px 48px 26px;
  }
  .grid {
    min-height: 0;
    padding: 4px 36px 40px;
    overflow: auto;
    scrollbar-gutter: stable both-edges;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(236px, 1fr));
    gap: 10px 22px;
    max-width: 1400px;
    margin: 0 auto;
    padding: 0;
    list-style: none;
  }
  .tile {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    gap: 16px;
    align-items: start;
    height: 100%;
    padding: 16px 16px 18px;
  }
  .tile-ic {
    margin-top: 1px;
    color: var(--accent);
  }
  .tile-ic :global(svg) {
    width: 30px;
    height: 30px;
  }
  .tile small {
    margin-top: 3px;
    line-height: 1.45;
  }
  @media (forced-colors: active) {
    .state i.ok,
    .state i.paused {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>
