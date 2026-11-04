<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { captures } from "../lib/capture.svelte";
  import { dumps } from "../lib/dumps.svelte";
  import { hwtests } from "../lib/tests.svelte";
  import { tweaks } from "../lib/tweaks.svelte";
  import { health } from "../lib/health.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { href, router } from "../lib/router.svelte";
  import { formatBytes } from "../screens/occurrence/format";
  import ConcernMark from "./ConcernMark.svelte";
  import Icon from "./Icon.svelte";

  const open = $derived(health.dto?.open.length ?? 0);
  const space = $derived(dumps.dto?.space_bytes ?? 0);
  const status = $derived(
    app.link === null
      ? { tone: "off", key: "ui.shell.status.connecting" }
      : app.paused
        ? { tone: "paused", key: "ui.shell.status.paused" }
        : captures.run !== null
          ? { tone: "ok", key: "ui.shell.status.capturing" }
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
  <div class="group" role="group" aria-labelledby="nav-follow">
    <p class="head" id="nav-follow">{t("ui.shell.nav.group.follow")}</p>
    <a class="nav" href={href.today()} aria-current={router.section === "today" ? "page" : undefined}>
      <Icon name="today" />{t("ui.shell.nav.today")}
    </a>
    <a class="nav" href={href.now()} aria-current={router.section === "now" ? "page" : undefined}>
      <Icon name="now" />{t("ui.shell.nav.now")}
    </a>
  </div>
  <div class="group" role="group" aria-labelledby="nav-care">
    <p class="head" id="nav-care">{t("ui.shell.nav.group.care")}</p>
    <a class="nav" href={href.health()} aria-current={router.section === "health" ? "page" : undefined}>
      <Icon name="health" />{t("ui.shell.nav.health")}
      {#if open}
        <span class="count">
          {#if health.critical}<ConcernMark mark="Critical" word={false} silent />{/if}
          <span aria-hidden="true">{open}</span>
          <span class="sr">
            {tn("ui.shell.nav.health.open", open)}{health.critical
              ? `, ${tn("ui.shell.nav.health.critical", health.critical)}`
              : ""}
          </span>
        </span>
      {/if}
    </a>
    <a class="nav" href={href.tests()} aria-current={router.section === "tests" ? "page" : undefined}>
      <Icon name="tests" />{t("ui.shell.nav.tests")}
      {#if hwtests.running || hwtests.dto?.scan}
        <span class="count"><ConcernMark mark="Running" word={false} silent /><span class="sr">{t("ui.shell.nav.tests.running")}</span></span>
      {/if}
    </a>
    <a class="nav" href={href.tweaks()} aria-current={router.section === "tweaks" ? "page" : undefined}>
      <Icon name="tweaks" />{t("ui.shell.nav.tweaks")}
      {#if tweaks.recommended}
        <span class="count">
          <span aria-hidden="true">{tweaks.recommended}</span>
          <span class="sr">{tn("ui.shell.nav.tweaks.signals", tweaks.recommended)}</span>
        </span>
      {/if}
    </a>
  </div>
  <div class="group" role="group" aria-labelledby="nav-investigate">
    <p class="head" id="nav-investigate">{t("ui.shell.nav.group.investigate")}</p>
    <a class="nav" href={href.dumps()} aria-current={router.section === "dumps" ? "page" : undefined}>
      <Icon name="dumps" />{t("ui.shell.nav.dumps")}
      {#if space}<span class="size">{formatBytes(space)}</span>{/if}
    </a>
    <a
      class="nav"
      href={href.captures()}
      aria-current={router.section === "captures" ? "page" : undefined}
    >
      <Icon name="captures" />{t("ui.shell.nav.captures")}
    </a>
  </div>
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
  .size {
    margin-left: auto;
    color: var(--ink2);
    font: 400 12px/18px var(--f);
    white-space: nowrap;
  }
  .count {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    padding: 0 7px;
    border-radius: 9px;
    background: var(--stroke2);
    color: var(--ink);
    font: 600 11px/18px var(--f);
  }
  .count :global(.cn) {
    gap: 0;
  }
  .count :global(svg) {
    width: 12px;
    height: 12px;
  }
  .group {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 2px;
  }
  .group + .group {
    margin-top: 10px;
  }
  .head {
    margin: 0;
    padding: 6px 12px 2px;
    color: var(--ink3);
    font: 600 12px/1.4 var(--f);
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
    .count {
      border: 1px solid CanvasText;
    }
    .nav[aria-current="page"]::before {
      forced-color-adjust: none;
      background: Highlight;
    }
  }
</style>
