import { expect, test } from "vitest";

import { marksDtoSamples, timelineDtoSamples } from "../../dev/fixtures/wire";
import { show } from "../../dev/test-mount";
import { t } from "../../lib/i18n.svelte";
import type { PcState } from "../../lib/types";
import { buildDay, dayStart, OPEN_TAIL_MS, type Run, settleTail } from "./day";
import States from "./States.svelte";

const NOON = new Date(2026, 0, 5, 12, 0, 0).getTime();
const MINUTE = 60_000;

/** One point a minute from midnight, in the states given, for the first minutes of the day. */
function dayOf(states: PcState[], now = NOON, away = false) {
  const start = dayStart(now);
  const line = {
    ...timelineDtoSamples[0],
    step_ms: MINUTE,
    t: states.map((_, i) => (start + i * MINUTE) / 1000),
    state: states,
  };
  return buildDay(now, line, { ...marksDtoSamples[0], occurrences: [], episodes: [], by_test: [], gaps: [] }, away);
}

test("a_hole_without_a_gap_is_drawn_as_no_reading_and_a_hole_inside_a_gap_as_suspended", () => {
  const day = dayOf(["calm", "no_reading", "no_reading", "suspended", "calm"]);

  expect(day.totals.no_reading).toBe(2 * MINUTE);
  expect(day.totals.suspended).toBe(MINUTE);
  expect(day.runs.map((r) => r.state)).toEqual(["calm", "no_reading", "suspended", "calm"]);
});

test("the_legend_has_a_no_reading_row_that_is_not_the_suspended_one", () => {
  const page = show(States, { day: dayOf(["calm", "no_reading", "suspended"]), level: "Simple" });

  const rows = [...page.querySelectorAll("button.legend")].map((b) => b.textContent ?? "");
  expect(rows).toHaveLength(5);
  expect(rows[3]).toContain(t("ui.common.state.suspended"));
  expect(rows[4]).toContain(t("ui.common.state.noreading"));
  expect(page.querySelectorAll("rect.run.s-no_reading")).toHaveLength(1);
  expect(page.querySelectorAll("rect.run.s-suspended")).toHaveLength(1);
});

test("the_newest_minutes_without_a_reading_are_left_empty_until_the_watcher_writes_them", () => {
  const now = NOON;
  const runs: Run[] = [
    { state: "calm", from: now - 60 * MINUTE, to: now - 30 * MINUTE },
    { state: "no_reading", from: now - 30 * MINUTE, to: now },
  ];

  const settled = settleTail(runs, now, false, now);

  expect(settled.at(-1)).toEqual({ state: "no_reading", from: now - 30 * MINUTE, to: now - OPEN_TAIL_MS });
  expect(settleTail([{ state: "no_reading", from: now - 3 * MINUTE, to: now }], now, false, now)).toEqual([]);
});

test("a_paused_or_stopped_watcher_makes_the_open_hole_an_absence_not_a_failure", () => {
  const now = NOON;
  const runs: Run[] = [
    { state: "suspended", from: now - 90 * MINUTE, to: now - 60 * MINUTE },
    { state: "no_reading", from: now - 60 * MINUTE, to: now },
  ];

  const away = settleTail(runs, now, true, now);

  expect(away).toEqual([{ state: "suspended", from: now - 90 * MINUTE, to: now }]);
  expect(settleTail(runs, now, false, now).at(-1)?.state).toBe("no_reading");
});

test("an_old_hole_stays_no_reading_whatever_the_watcher_does_now", () => {
  const now = NOON;
  const runs: Run[] = [{ state: "no_reading", from: now - 120 * MINUTE, to: now - 60 * MINUTE }];

  expect(settleTail(runs, now, true, now)).toEqual(runs);
});
