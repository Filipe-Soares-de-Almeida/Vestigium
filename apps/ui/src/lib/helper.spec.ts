import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach, expect, test } from "vitest";

import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import CaptureStrip from "../components/CaptureStrip.svelte";
import { captures } from "./capture.svelte";
import { dumps } from "./dumps.svelte";
import { buildGroups } from "./groups";
import { health } from "./health.svelte";
import { helperJobs } from "./helper.svelte";
import { t } from "./i18n.svelte";
import { router } from "./router.svelte";
import type { CaptureFinished, CaptureRun } from "./types";

const RUN: CaptureRun = { started_ms: Date.now() - 60_000, ends_ms: Date.now() + 240_000, own: true };

beforeEach(() => {
  router.commit({ screen: "captures", forOccurrence: null }, null);
  helperJobs.last = {};
  helperJobs.asking = null;
  captures.run = null;
  captures.cancelling = false;
  dumps.reading = "idle";
  health.phase = "idle";
});

afterEach(() => {
  helperJobs.last = {};
  helperJobs.asking = null;
  captures.run = null;
  captures.cancelling = false;
});

test("the_capture_strip_offers_a_cancel_that_asks_before_it_acts", () => {
  captures.run = RUN;
  const page = show(CaptureStrip);

  const cancel = [...page.querySelectorAll("button")].find((b) => b.textContent === t("ui.helper.cancel.capture"));
  expect(cancel).toBeDefined();
  cancel!.click();

  expect(helperJobs.asking).toBe("capture");
});

test("cancelling_a_capture_tells_the_window_to_ask_the_helper_and_waits_for_its_end", async () => {
  const calls = answer({ cancel_capture: () => null });
  captures.run = RUN;
  const page = show(CaptureStrip);

  await captures.cancel();

  expect(calls.of("cancel_capture")).toHaveLength(1);
  expect(captures.cancelling).toBe(true);
  await until(() => page.textContent?.includes(t("ui.helper.cancelling.note")) === true, "the note");
  const labels = [...page.querySelectorAll("button")].map((b) => b.textContent?.trim());
  expect(labels).toContain(t("ui.helper.cancelling"));
  expect(labels).not.toContain(t("ui.helper.cancel.capture"));
});

test("a_cancel_that_could_not_be_sent_leaves_the_capture_running_and_says_so", async () => {
  answer({ cancel_capture: () => Promise.reject(refusal("failed")) });
  captures.run = RUN;

  await captures.cancel();

  expect(captures.cancelling).toBe(false);
});

/** Starts listening and delivers `done` as the helper's end. */
async function ended(done: CaptureFinished): Promise<() => void> {
  mockIPC(
    (cmd) => {
      if (cmd === "import_captures") return [];
      if (cmd === "captures") return [];
      return null;
    },
    { shouldMockEvents: true },
  );
  const stop = captures.watch();
  await emit("capture-finished", done);
  return stop;
}

test("a_cancelled_capture_keeps_a_line_and_no_problem", async () => {
  captures.run = RUN;
  captures.cancelling = true;
  const stop = await ended({ ids: [], exit_code: 5, ended: "Cancelled" });
  await until(() => captures.run === null && !captures.cancelling, "the end");

  expect(helperJobs.last.capture?.kind).toBe("cancelled");
  expect(helperJobs.unseen("capture")).toBe(false);
  expect(captures.problem).toBeNull();
  stop();
});

test("a_capture_the_window_gave_up_on_is_told_once_and_marks_its_side_item", async () => {
  captures.run = RUN;
  const stop = await ended({ ids: [], exit_code: null, ended: "TimedOut" });
  await until(() => helperJobs.last.capture !== undefined, "the timeout");

  expect(helperJobs.unseen("capture")).toBe(true);
  expect(captures.problem).toBeNull();
  const item = buildGroups().find((g) => g.key === "captures")!.items[0];
  expect(item.status).toBe("attention");
  expect(item.statusWord).toBe(t("ui.helper.timeout.title"));

  helperJobs.see("capture");
  expect(buildGroups().find((g) => g.key === "captures")!.items[0].status).toBeUndefined();
  stop();
});

test("a_capture_the_helper_saved_is_finished_even_if_a_cancel_came_late", async () => {
  captures.run = RUN;
  const stop = await ended({ ids: [4], exit_code: 0, ended: "Cancelled" });
  await until(() => captures.finished === 4, "the finished capture");

  expect(helperJobs.last.capture).toBeUndefined();
  stop();
});

test("a_helper_that_ended_on_its_own_without_a_result_is_still_an_interruption", async () => {
  captures.run = RUN;
  const stop = await ended({ ids: [], exit_code: 1, ended: "Done" });
  await until(() => captures.problem === "interrupted", "the problem");

  expect(helperJobs.last.capture).toBeUndefined();
  stop();
});

test("a_new_capture_forgets_how_the_last_one_ended", async () => {
  helperJobs.record("capture", "timeout");
  answer({ start_capture: () => RUN });

  await captures.begin(10);

  expect(helperJobs.last.capture).toBeUndefined();
  expect(captures.lastMinutes).toBe(10);
});

test("a_dump_read_is_cancelled_straight_away_with_no_question", async () => {
  const calls = answer({
    start_dump_scan: () => Promise.reject(refusal("cancelled")),
    cancel_helper_read: () => null,
  });

  const reading = dumps.read();
  await dumps.cancelRead();
  await reading;

  expect(calls.of("cancel_helper_read")).toEqual([{ read: "dumps" }]);
  expect(helperJobs.last.dumps?.kind).toBe("cancelled");
  expect(helperJobs.asking).toBeNull();
  expect(dumps.reading).toBe("idle");
});

test("a_dump_read_that_hits_its_limit_is_a_timeout_not_a_failure", async () => {
  answer({ start_dump_scan: () => Promise.reject(refusal("helper_timed_out")) });

  await dumps.read();

  expect(helperJobs.unseen("dumps")).toBe(true);
  expect(dumps.reading).toBe("idle");
});

test("a_full_scan_is_cancelled_and_timed_out_the_same_way", async () => {
  const calls = answer({
    start_scan: () => Promise.reject(refusal("cancelled")),
    cancel_helper_read: () => null,
  });
  await health.scan();
  await health.cancelScan();
  expect(calls.of("cancel_helper_read")).toEqual([{ read: "scan" }]);
  expect(helperJobs.last.health?.kind).toBe("cancelled");
  expect(health.phase).toBe("idle");

  answer({ start_scan: () => Promise.reject(refusal("helper_timed_out")) });
  await health.scan();
  expect(helperJobs.unseen("health")).toBe(true);
  expect(buildGroups().find((g) => g.key === "health")!.items[0].status).toBe("attention");
});

test("a_scan_that_failed_for_another_reason_stays_a_failure", async () => {
  answer({ start_scan: () => Promise.reject(refusal("failed")) });

  await health.scan();

  expect(health.phase).toBe("failed");
  expect(helperJobs.last.health).toBeUndefined();
});
