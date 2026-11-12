import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { episodeSamples, marksDtoSamples, timelineDtoSamples } from "../dev/fixtures/wire";
import { app } from "../lib/app.svelte";
import { t } from "../lib/i18n.svelte";
import Today from "./Today.svelte";

const NOON = new Date(2026, 0, 5, 12, 0, 0).getTime();
const MIDNIGHT = new Date(2026, 0, 5, 0, 0, 0).getTime();

/** Two minutes of today, so the day has something to draw. */
const line = {
  ...timelineDtoSamples[0],
  t: [MIDNIGHT / 1000 + 3600, MIDNIGHT / 1000 + 3601],
  state: ["calm", "busy"] as ("calm" | "busy")[],
};

/** Two disk episodes that began in the same millisecond. */
const sameStart = [
  { ...episodeSamples[3], start_ms: NOON - 600_000, end_ms: NOON - 300_000 },
  { ...episodeSamples[3], start_ms: NOON - 600_000, end_ms: NOON - 240_000 },
];

const marks = { ...marksDtoSamples[0], occurrences: [], episodes: sameStart, by_test: [false, false], gaps: [] };

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOON);
  app.history = "ready";
});

afterEach(() => {
  vi.useRealTimers();
});

test("moments_lists_every_episode_that_started_in_the_same_millisecond", async () => {
  answer({ timeline: () => line, marks: () => marks });
  const page = show(Today);
  await until(() => page.querySelectorAll("a.row").length > 0, "the moments list");
  expect(page.querySelectorAll("a.row")).toHaveLength(2);
});

test("a_failed_first_read_shows_the_error_and_try_again_loads_the_day", async () => {
  let failing = true;
  const calls = answer({
    timeline: () => (failing ? Promise.reject(refusal("store")) : line),
    marks: () => marks,
  });
  const page = show(Today);
  await until(() => page.querySelector(".failure") !== null, "the error view");
  expect(page.querySelector(".failure")!.textContent).toContain(t("ui.common.error.store"));
  expect(page.textContent).not.toContain(t("ui.today.moments.none"));

  failing = false;
  const retry = [...page.querySelectorAll("button")].find((b) => b.textContent === t("ui.common.retry"));
  expect(retry).toBeDefined();
  retry!.click();
  await until(() => page.querySelector(".failure") === null, "the error view to go away");
  expect(calls.of("timeline").length).toBe(2);
});
