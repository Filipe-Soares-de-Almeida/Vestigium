import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { episodeSamples, marksDtoSamples } from "../dev/fixtures/wire";
import { answer, refusal } from "../dev/test-ipc";
import { buildGroups } from "./groups";
import { t, tn } from "./i18n.svelte";
import { today } from "./today.svelte";

const NOON = new Date(2026, 0, 5, 12, 0, 0).getTime();

/** Two pressure episodes of today and no occurrence. */
const episodes = [
  { ...episodeSamples[3], start_ms: NOON - 600_000, end_ms: NOON - 300_000 },
  { ...episodeSamples[3], start_ms: NOON - 500_000, end_ms: NOON - 240_000 },
];
const busyDay = { ...marksDtoSamples[0], occurrences: [], episodes, by_test: [false, false], gaps: [] };

beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(NOON);
  today.loaded = false;
  today.failed = false;
  today.occurrences = [];
  today.periods = 0;
});

afterEach(() => {
  vi.useRealTimers();
  today.loaded = false;
  today.failed = false;
});

const freezes = () => buildGroups().find((group) => group.key === "freezes")!;

test("a_failed_marks_read_gives_the_freezes_group_no_counts", async () => {
  answer({ marks: () => Promise.reject(refusal("store")) });
  await today.load();

  expect(today.failed).toBe(true);
  expect(freezes().sub).toBe(t("ui.group.freezes.sub"));
  expect(freezes().items[0].meta).toBeUndefined();
});

test("a_marks_read_still_under_way_gives_the_freezes_group_no_counts_and_no_failure", () => {
  answer({ marks: () => new Promise(() => {}) });
  void today.load();

  expect(today.failed).toBe(false);
  expect(freezes().sub).toBe(t("ui.group.freezes.sub"));
  expect(freezes().items[0].meta).toBeUndefined();
});

test("a_good_read_gives_the_freezes_group_its_counts_even_when_the_day_is_empty", async () => {
  answer({ marks: () => ({ ...busyDay, episodes: [], by_test: [] }) });
  await today.load();

  expect(freezes().sub).toBe(
    t("ui.group.freezes.sub_n", { occurrences: t("ui.events.occurrences.none"), periods: t("ui.events.periods.none") }),
  );
  expect(freezes().items[0].meta).toBe("0");
});

test("a_failed_refresh_keeps_the_last_good_counts", async () => {
  answer({ marks: () => busyDay });
  await today.load();
  const good = freezes();
  expect(good.sub).toBe(
    t("ui.group.freezes.sub_n", { occurrences: t("ui.events.occurrences.none"), periods: tn("ui.events.periods", 2) }),
  );
  expect(good.items[0].meta).toBe("2");

  answer({ marks: () => Promise.reject(refusal("store")) });
  await today.load();

  expect(today.failed).toBe(false);
  expect(freezes().sub).toBe(good.sub);
  expect(freezes().items[0].meta).toBe("2");
});

test("a_good_read_after_a_failed_one_clears_the_failure", async () => {
  answer({ marks: () => Promise.reject(refusal("store")) });
  await today.load();
  expect(today.failed).toBe(true);

  answer({ marks: () => busyDay });
  await today.load();

  expect(today.failed).toBe(false);
  expect(freezes().items[0].meta).toBe("2");
});
