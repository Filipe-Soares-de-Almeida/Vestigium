import { afterEach, beforeEach, expect, test } from "vitest";

import { agentStatusSamples, healthDtoSamples } from "../dev/fixtures/wire";
import { show } from "../dev/test-mount";
import { app } from "../lib/app.svelte";
import { health } from "../lib/health.svelte";
import { t } from "../lib/i18n.svelte";
import { passState } from "../lib/monitor";
import type { FindingKind, HealthDto, MonitorStatus } from "../lib/types";
import { AREAS, areaState } from "./health/areas";
import { checkText } from "./health/words";
import Health from "./Health.svelte";

const NOW = Date.now();
const HOUR = 3_600_000;

/** A health page with nothing open: every area is only as sure as the checks behind it. */
const quiet: HealthDto = { ...healthDtoSamples[0], open: [], closed: [], last_scan_ms: NOW - 24 * HOUR };

function link(monitor: Partial<MonitorStatus>): void {
  const base = agentStatusSamples[1];
  app.link = {
    link: "running",
    ...base,
    started_at_ms: NOW - 10 * HOUR,
    last_sample_ms: NOW - 1000,
    monitor: { ...base.monitor, resumed_ms: NOW - 10 * HOUR, ...monitor },
  };
}

beforeEach(() => {
  health.dto = quiet;
  health.error = null;
});

afterEach(() => {
  app.link = null;
  health.dto = null;
});

const protection = AREAS.find((a) => a.id === "protection")!;
const disks = AREAS.find((a) => a.id === "disks")!;

test("an_unread_antivirus_query_is_not_a_fine_check", () => {
  expect(areaState(protection, quiet).mark).toBe("Ok");

  const unread = areaState(protection, quiet, { stale: false, since: null, silent: false, unread: ["TwoAntivirus" as FindingKind] });

  expect(unread.mark).toBe("Unknown");
  expect(unread.why).toBe("unread");
  expect(checkText(unread.checks[0])).toBe(t("ui.health.check.unread"));
});

test("a_pass_that_stopped_finishing_makes_the_checks_unknown_with_the_time_of_the_last_good_one", () => {
  link({ health_attempt_ms: NOW - 60_000, health_ok_ms: NOW - 4 * HOUR });
  const pass = passState(app.link, NOW);

  expect(pass).toMatchObject({ stale: true, since: NOW - 4 * HOUR });
  const state = areaState(disks, quiet, pass);
  expect(state.mark).toBe("Unknown");
  expect(state.why).toBe("stale");
  expect(checkText(state.checks[0], pass)).toContain(t("ui.health.check.unread_since", { time: "" }).trim().slice(0, 8));
});

test("a_check_that_needs_a_scan_stays_as_it_was_whatever_the_pass_says", () => {
  const startup = AREAS.find((a) => a.id === "startup")!;

  expect(areaState(startup, { ...quiet, last_scan_ms: null }).why).toBe("scan");
  expect(areaState(startup, quiet, { stale: true, since: NOW - 4 * HOUR, silent: false, unread: [] }).mark).toBe("Ok");
});

test("a_failing_pass_shows_the_last_good_time_and_never_all_normal", () => {
  link({ health_attempt_ms: NOW - 60_000, health_ok_ms: NOW - 4 * HOUR });
  const page = show(Health, { area: null });

  const text = page.textContent ?? "";
  expect(text).toContain(t("ui.health.lede.unverified.rest"));
  expect(text).not.toContain(t("ui.health.lede.ok"));
  expect(page.querySelector(".status.unverified")).not.toBeNull();
  expect(page.querySelector("header, .head, h1")?.parentElement?.textContent).toContain(t("ui.health.scan.failed", { when: "", attempt: "" }).slice(0, 14));
});

test("a_pass_that_finished_lately_still_says_all_normal_with_its_time", () => {
  link({ health_attempt_ms: NOW - 60_000, health_ok_ms: NOW - 60_000 });
  const page = show(Health, { area: null });

  const text = page.textContent ?? "";
  expect(text).toContain(t("ui.health.lede.ok"));
  expect(text).toContain(t("ui.health.scan.last_ok", { when: "" }).slice(0, 12));
  expect(page.querySelector(".status.unverified")).toBeNull();
});

test("an_agent_that_is_not_running_adds_nothing_to_the_page", () => {
  app.link = { link: "stopped" };
  const page = show(Health, { area: null });

  expect(page.textContent).toContain(t("ui.health.lede.ok"));
  expect(page.querySelector(".status.unverified")).toBeNull();
});

test("a_silent_monitor_says_the_list_is_the_last_saved_one", () => {
  app.link = { link: "silent", seconds: 3 };
  const page = show(Health, { area: null });

  expect(page.textContent).toContain(t("ui.health.lede.silent.rest"));
  expect(page.textContent).not.toContain(t("ui.health.lede.ok"));
});
