import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { captures } from "../lib/capture.svelte";
import { buildGroups, hasLive } from "../lib/groups";
import { helperJobs } from "../lib/helper.svelte";
import { setLang, t } from "../lib/i18n.svelte";
import { router } from "../lib/router.svelte";
import type { CaptureRun } from "../lib/types";
import CaptureStrip from "./CaptureStrip.svelte";
import Banner from "./Banner.svelte";
import Start from "../screens/captures/Start.svelte";

const MIN = 60_000;
const OWN: CaptureRun = { started_ms: Date.now() - 2 * MIN, ends_ms: Date.now() + 3 * MIN, own: true };
const SINCE: CaptureRun = { started_ms: Date.now() - 7 * MIN, ends_ms: null, own: false };
const EXISTS: CaptureRun = { started_ms: null, ends_ms: null, own: false };

function reset(): void {
  captures.run = null;
  captures.refused = false;
  captures.finished = null;
  captures.entries = null;
  captures.cancelling = false;
  helperJobs.last = {};
  helperJobs.asking = null;
  setLang("en");
  router.commit({ screen: "today" } as never, null);
}

beforeEach(reset);
afterEach(() => {
  reset();
  vi.useRealTimers();
});

const text = (page: HTMLElement) => page.textContent?.replace(/\s+/g, " ").trim() ?? "";

test("no_capture_and_nothing_to_tell_shows_no_strip", () => {
  const page = show(CaptureStrip);
  expect(page.querySelector(".strip")).toBeNull();
});

test("a_capture_started_here_shows_the_clock_the_bar_and_when_it_ends", () => {
  captures.run = OWN;
  const page = show(CaptureStrip);

  expect(text(page)).toContain(t("ui.captures.strip.title"));
  expect(text(page)).toContain("2:00 of 5:00");
  expect(page.querySelector(".pbar:not(.unknown)")).not.toBeNull();
  expect(text(page)).not.toContain(t("ui.captures.strip.who"));
});

test("a_capture_another_window_started_says_since_when_and_that_the_end_is_unknown", () => {
  captures.run = SINCE;
  const page = show(CaptureStrip);

  expect(text(page)).toContain("For 7:00");
  expect(text(page)).toContain("The end is not known.");
  expect(text(page)).toContain(t("ui.captures.strip.who"));
  expect(page.querySelector(".pbar.unknown")).not.toBeNull();
});

test("a_capture_whose_start_is_unknown_says_only_that_it_started_earlier", () => {
  captures.run = EXISTS;
  const page = show(CaptureStrip);

  expect(text(page)).toContain(t("ui.captures.strip.earlier"));
  expect(text(page)).not.toContain("For ");
});

test("the_strip_reads_in_portuguese", () => {
  setLang("pt-BR");
  captures.run = SINCE;
  const page = show(CaptureStrip);

  expect(text(page)).toContain(t("ui.captures.strip.title"));
  expect(text(page)).not.toContain("Capture in progress");
  expect(text(page)).toContain("7:00");
});

test("another_screen_gets_a_link_to_the_captures_and_the_captures_screen_gets_none", () => {
  captures.run = SINCE;
  const elsewhere = show(CaptureStrip);
  expect(elsewhere.querySelector("a")?.textContent).toBe(t("ui.captures.strip.see"));

  router.commit({ screen: "captures", forOccurrence: null }, null);
  const here = show(CaptureStrip);
  expect(here.querySelector("a")).toBeNull();
  expect(here.querySelector("button")).toBeNull();
});

test("a_refused_second_capture_changes_the_text_and_can_be_dismissed", () => {
  captures.run = SINCE;
  captures.refused = true;
  const page = show(CaptureStrip);

  expect(page.querySelector('[role="alert"]')).not.toBeNull();
  expect(text(page)).toContain(t("ui.captures.strip.refused"));
  expect(text(page)).toContain(t("ui.captures.strip.refused.body"));
  expect(text(page)).not.toContain(t("ui.captures.strip.title"));

  page.querySelector("button")!.click();
  expect(captures.refused).toBe(false);
});

test("a_finished_capture_offers_its_result_and_can_be_closed", () => {
  captures.finished = 7;
  const page = show(CaptureStrip);

  expect(text(page)).toContain(t("ui.captures.strip.done"));
  const link = page.querySelector("a");
  expect(link?.textContent).toBe(t("ui.captures.finished.open"));
  expect(link?.getAttribute("href")).toContain("7");

  [...page.querySelectorAll("button")].find((b) => b.textContent === t("ui.captures.strip.close"))!.click();
  expect(captures.finished).toBeNull();
});

test("the_strip_stacks_below_the_monitors_banner_without_replacing_it", () => {
  captures.run = SINCE;
  const banner = show(Banner);
  const strip = show(CaptureStrip);

  expect(banner.querySelector(".banner")).toBeNull();
  expect(strip.querySelector(".strip")).not.toBeNull();
});

test("the_captures_entry_carries_an_information_mark_while_a_capture_runs", () => {
  const find = () => buildGroups().find((g) => g.key === "captures")!;
  expect(find().items[0].status).toBeUndefined();

  captures.run = SINCE;
  expect(find().items[0].status).toBe("info");
  expect(find().items[0].statusWord).toBe(t("ui.captures.nav.running"));
  expect(hasLive(find())).toBe(true);

  helperJobs.record("capture", "timeout");
  expect(find().items[0].status).toBe("attention");
  expect(hasLive(find())).toBe(false);
});

test("start_waits_for_the_capture_and_says_so_while_the_form_stays", async () => {
  const calls = answer({ start_capture: () => OWN });
  captures.run = SINCE;
  const page = show(Start, { context: null });

  const start = page.querySelector<HTMLButtonElement>(".btn.accent")!;
  expect(start.getAttribute("aria-disabled")).toBe("true");
  expect(text(page)).toContain(t("ui.captures.start.blocked"));
  start.click();
  await new Promise((resolve) => setTimeout(resolve, 10));
  expect(calls.of("start_capture")).toHaveLength(0);
});

test("a_start_the_window_refuses_keeps_the_running_capture_and_says_a_second_was_refused", async () => {
  answer({
    start_capture: () => Promise.reject(refusal("capture_running")),
    capture_run: () => SINCE,
  });

  await captures.begin(5);

  expect(captures.refused).toBe(true);
  expect(captures.run).toEqual(SINCE);
  expect(captures.problem).toBeNull();
});

test("a_helper_that_found_a_capture_running_is_a_refusal_not_an_interruption", async () => {
  mockIPC(
    (cmd) => {
      if (cmd === "capture_run") return SINCE;
      if (cmd === "import_captures" || cmd === "captures") return [];
      return null;
    },
    { shouldMockEvents: true },
  );
  captures.run = OWN;
  const stop = captures.watch();

  await emit("capture-finished", { ids: [], exit_code: 4, ended: "Refused" });
  await until(() => captures.refused, "the refusal");

  expect(captures.run).toEqual(SINCE);
  expect(captures.problem).toBeNull();
  stop();
});

test("a_capture_another_window_started_is_seen_ending_and_its_result_offered", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  let present = true;
  mockIPC(
    (cmd) => {
      if (cmd === "capture_run") return present ? SINCE : null;
      if (cmd === "import_captures") return present ? [] : [9];
      if (cmd === "captures") return [];
      return null;
    },
    { shouldMockEvents: true },
  );
  await captures.sync();
  expect(captures.run).toEqual(SINCE);
  expect(captures.finished).toBeNull();

  present = false;
  await vi.advanceTimersByTimeAsync(5100);
  await vi.waitFor(() => expect(captures.finished).toBe(9));

  expect(captures.run).toBeNull();
});
