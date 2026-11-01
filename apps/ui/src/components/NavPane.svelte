<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { t } from "../lib/i18n.svelte";
  import { href, router } from "../lib/router.svelte";
  import Icon from "./Icon.svelte";

  const status = $derived(
    app.link === null
      ? { tone: "off", key: "ui.shell.status.connecting" }
      : app.paused
        ? { tone: "paused", key: "ui.shell.status.paused" }
        : app.running && app.link.link === "running" && app.link.sensitive_until_ms !== null
          ? { tone: "ok", key: "ui.shell.status.sensitive" }
          : app.running
            ? { tone: "ok", key: "ui.shell.status.watching" }
            : app.stopped
              ? { tone: "off", key: "ui.shell.status.stopped" }
              : { tone: "off", key: "ui.shell.status.unreachable" },
  );
</script>

<nav class="pane" aria-label={t("ui.shell.nav.label")}>
  <a class="nav" href={href.today()} aria-current={router.section === "today" ? "page" : undefined}>
    <Icon name="today" />{t("ui.shell.nav.today")}
  </a>
  <a class="nav" href={href.now()} aria-current={router.section === "now" ? "page" : undefined}>
    <Icon name="now" />{t("ui.shell.nav.now")}
  </a>
  <div class="sep" role="separator"></div>
  <span class="nav soon-item">
    <Icon name="health" />{t("ui.shell.nav.health")}<span class="soon">{t("ui.shell.nav.soon")}</span>
  </span>
  <span class="nav soon-item">
    <Icon name="dumps" />{t("ui.shell.nav.dumps")}<span class="soon">{t("ui.shell.nav.soon")}</span>
  </span>
  <p class="foot"><i class={status.tone} aria-hidden="true"></i>{t(status.key)}</p>
  <a
    class="nav"
    href={href.settings()}
    aria-current={router.section === "settings" ? "page" : undefined}
  >
    <Icon name="settings" />{t("ui.shell.nav.settings")}
  </a>
</nav>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    padding: 4px 6px 10px;
    overflow: auto;
  }
  .nav {
    position: relative;
    display: flex;
    flex: none;
    align-items: center;
    gap: 14px;
    min-height: 38px;
    padding: 0 12px;
    border-radius: 6px;
    color: var(--ink);
    font: 400 14px/1.5 var(--f);
    text-decoration: none;
  }
  .nav :global(svg) {
    flex: none;
  }
  a.nav:hover {
    background: var(--stroke2);
  }
  .nav[aria-current="page"] {
    background: var(--stroke2);
    font-weight: 600;
  }
  .nav[aria-current="page"]::before {
    content: "";
    position: absolute;
    left: 0;
    top: 11px;
    bottom: 11px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }
  .soon-item {
    color: var(--ink3);
  }
  .soon {
    margin-left: auto;
    padding: 0 7px;
    border-radius: 9px;
    background: var(--stroke2);
    color: var(--ink2);
    font: 400 11px/18px var(--f);
    white-space: nowrap;
  }
  .sep {
    flex: none;
    margin: 6px 10px;
    border-top: 1px solid var(--stroke2);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: auto 0 0;
    padding: 10px 12px;
    color: var(--ink2);
    font: 400 12.5px/1.35 var(--f);
  }
  .foot i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .foot i.ok {
    background: var(--ok);
  }
  .foot i.paused {
    background: var(--press);
  }
  .foot i.off {
    border: 1.5px solid var(--ink3);
  }
  @media (forced-colors: active) {
    .foot i,
    .foot i.ok,
    .foot i.paused {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .foot i.off {
      background: Canvas;
      border-color: CanvasText;
    }
    .soon-item {
      color: GrayText;
    }
    .nav[aria-current="page"]::before {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
