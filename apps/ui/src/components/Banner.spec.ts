import { afterEach, expect, test } from "vitest";

import { agentStatusSamples } from "../dev/fixtures/wire";
import { show } from "../dev/test-mount";
import { app } from "../lib/app.svelte";
import { setLang, t } from "../lib/i18n.svelte";
import { router } from "../lib/router.svelte";
import { shellStatus } from "../lib/status";
import type { MonitorStatus } from "../lib/types";
import Banner from "./Banner.svelte";

const NOW = Date.now();
const HOUR = 3_600_000;

/** A running watcher that is reading, with `monitor` overriding what it says about itself. */
function running(monitor: Partial<MonitorStatus> = {}, rest: Record<string, unknown> = {}): void {
  const base = agentStatusSamples[1];
  app.link = {
    link: "running",
    ...base,
    started_at_ms: NOW - 5 * HOUR,
    last_sample_ms: NOW - 1000,
    monitor: { ...base.monitor, resumed_ms: NOW - 5 * HOUR, ...monitor },
    ...rest,
  };
}

afterEach(() => {
  app.link = null;
  app.settings = null;
  router.commit({ screen: "home" }, null);
});

test("a_reading_monitor_shows_no_banner_and_says_monitoring", () => {
  running();
  const page = show(Banner);
  expect(page.querySelector(".banner")).toBeNull();
  expect(shellStatus().text).toContain(t("ui.shell.status.watching"));
  expect(shellStatus().tone).toBe("ok");
});

test("no_reading_says_so_in_a_neutral_banner_and_the_status_line_stops_saying_monitoring", () => {
  running({ state: "no_reading", no_reading_since_ms: NOW - 82 * 60_000 }, { last_sample_ms: NOW - 82 * 60_000 });
  const page = show(Banner);

  const banner = page.querySelector(".banner");
  expect(banner?.classList.contains("neutral")).toBe(true);
  expect(banner?.textContent).toContain(t("ui.shell.no_reading.title"));
  expect(banner?.textContent).toContain("1 h 22");
  expect([...page.querySelectorAll("button")].some((b) => b.textContent === t("ui.common.retry"))).toBe(true);
  expect(shellStatus().tone).toBe("no_reading");
  expect(shellStatus().text).not.toContain(t("ui.shell.status.watching"));
});

test("the_no_reading_banner_reads_in_portuguese", () => {
  setLang("pt-BR");
  running({ state: "no_reading", no_reading_since_ms: NOW - 10 * 60_000 });
  const page = show(Banner);
  expect(page.querySelector(".banner")?.textContent).toContain("Sem leitura.");
  expect(shellStatus().text).toContain("Sem leitura desde");
});

test("a_failing_source_is_named_and_the_error_shows_only_at_the_technical_level", () => {
  const failing = [{ source: "counters" as const, since_ms: NOW - HOUR, error: "PdhCollectQueryData failed" }];
  running({ state: "degraded", failing });
  const simple = show(Banner);
  expect(simple.querySelector(".banner")?.textContent).toContain(t("ui.monitor.source.counters"));
  expect(simple.querySelector(".mono")).toBeNull();
  expect(shellStatus().tone).toBe("partial");

  app.settings = { settings: { detail_level: "Technical" }, autostart: null, editable: false } as never;
  const technical = show(Banner);
  expect(technical.querySelector(".mono")?.textContent).toContain("PdhCollectQueryData failed");
});

test("paused_and_suspended_do_not_read_as_no_reading", () => {
  running({ state: "watching" }, { paused: true, paused_until_ms: null });
  const page = show(Banner);
  expect(page.querySelector(".banner")?.textContent).toContain(t("ui.shell.paused.title"));
  expect(page.querySelector(".banner")?.classList.contains("neutral")).toBe(false);
  expect(shellStatus().tone).toBe("paused");
});

test("an_unverified_health_check_shows_a_banner_only_on_the_health_screen", () => {
  running({ health_attempt_ms: NOW - 60_000, health_ok_ms: NOW - 4 * HOUR });
  expect(show(Banner).querySelector(".banner")).toBeNull();
  expect(shellStatus().tone).toBe("unverified");

  router.commit({ screen: "health", area: null }, null);
  const onHealth = show(Banner);
  expect(onHealth.querySelector(".banner")?.textContent).toContain(t("ui.shell.unverified.title"));
  expect(onHealth.querySelectorAll("button")).toHaveLength(0);
});

test("a_silent_watcher_gets_its_own_neutral_banner_not_the_version_one", () => {
  app.link = { link: "silent", seconds: 3 };
  const page = show(Banner);

  const banner = page.querySelector(".banner");
  expect(banner?.classList.contains("neutral")).toBe(true);
  expect(banner?.textContent).toContain(t("ui.shell.silent.title"));
  expect(banner?.textContent).toContain("3 s");
  expect(banner?.textContent).not.toContain(t("ui.shell.unreachable.title"));
  expect(shellStatus().tone).toBe("silent");
  expect(shellStatus().text).toBe(t("ui.shell.status.silent"));
});
