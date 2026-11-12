import { afterEach, beforeEach, expect, test } from "vitest";

import { marksDtoSamples } from "../dev/fixtures/wire";
import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { t } from "../lib/i18n.svelte";
import { today } from "../lib/today.svelte";
import Home from "./Home.svelte";

const none = { ...marksDtoSamples[0], occurrences: [], episodes: [], by_test: [], gaps: [] };

beforeEach(() => {
  today.loaded = false;
  today.failed = false;
  today.occurrences = [];
  today.periods = 0;
});

afterEach(() => {
  today.loaded = false;
  today.failed = false;
});

/** The text of the card named `name`. */
function card(page: HTMLElement, name: string): HTMLElement {
  const found = [...page.querySelectorAll<HTMLElement>("a.card")].find((a) => a.querySelector("b")?.textContent === name);
  if (!found) throw new Error(`no card ${name}`);
  return found;
}

test("a_failed_read_of_todays_marks_is_not_no_freezes_with_a_green_mark", async () => {
  answer({ marks: () => Promise.reject(refusal("store")) });
  await today.load();
  const page = show(Home);

  const activity = card(page, t("ui.group.activity"));
  expect(activity.textContent).toContain(t("ui.home.activity.marks_failed"));
  expect(activity.textContent).not.toContain(t("ui.home.activity.none"));
  expect(activity.querySelector(".status.no_reading")).not.toBeNull();
  expect(activity.querySelector(".status.ok")).toBeNull();
});

test("a_first_read_still_under_way_shows_neither_none_nor_a_failure", () => {
  const page = show(Home);

  const activity = card(page, t("ui.group.activity"));
  expect(activity.textContent).not.toContain(t("ui.home.activity.none"));
  expect(activity.textContent).not.toContain(t("ui.home.activity.marks_failed"));
  expect(activity.querySelector(".status")).toBeNull();
});

test("a_day_with_nothing_still_says_no_freezes_and_a_later_failure_keeps_it", async () => {
  answer({ marks: () => none });
  await today.load();
  const page = show(Home);
  await until(() => card(page, t("ui.group.activity")).textContent?.includes(t("ui.home.activity.none")) ?? false, "the empty day");
  expect(card(page, t("ui.group.activity")).querySelector(".status.ok")).not.toBeNull();

  answer({ marks: () => Promise.reject(refusal("store")) });
  await today.load();

  expect(today.failed).toBe(false);
  expect(card(page, t("ui.group.activity")).textContent).toContain(t("ui.home.activity.none"));
});

test("tweaks_that_could_not_be_read_do_not_say_nothing_is_recommended", async () => {
  const page = show(Home);
  const { tweaks } = await import("../lib/tweaks.svelte");
  answer({ tweaks: () => Promise.reject(refusal("store")) });
  await tweaks.load();

  await until(() => card(page, t("ui.group.tweaks")).textContent?.includes(t("ui.home.tweaks.failed")) ?? false, "the failed tweaks");
  expect(card(page, t("ui.group.tweaks")).textContent).not.toContain(t("ui.home.tweaks.none"));
  tweaks.error = null;
});
