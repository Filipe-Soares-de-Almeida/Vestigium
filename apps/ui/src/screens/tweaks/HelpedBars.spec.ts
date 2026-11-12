import { expect, test } from "vitest";

import { show } from "../../dev/test-mount";
import { helpedSamples } from "../../dev/fixtures/wire";
import { setLang } from "../../lib/i18n.svelte";
import type { Helped } from "../../lib/types";
import HelpedBars from "./HelpedBars.svelte";

const HOUR = 3_600_000;

const base: Helped = {
  ...helpedSamples[0],
  days: 14,
  before: 18,
  after: 4,
  before_ms: 168 * HOUR,
  after_ms: 160 * HOUR,
  verdict: "Fewer",
  min_events: 5,
  min_hours: 12,
};

const widths = (page: HTMLElement) => [...page.querySelectorAll("i")].map((bar) => parseFloat(bar.style.width));

test("bars_measure_the_rate_per_monitored_hour_from_zero", () => {
  const page = show(HelpedBars, { helped: base, metric: "stutters" });

  const [before, after] = widths(page);
  expect(before).toBeCloseTo(100, 5);
  expect(after).toBeCloseTo((4 / 160 / (18 / 168)) * 100, 3);
  expect(page.textContent).toContain("0.11 per hour");
  expect(page.textContent).toContain("0.03 per hour");
  expect(page.textContent).toContain("18 in 168 h");
  expect(page.textContent).toContain("Fewer since the change");
  expect(page.querySelector(".lacking")).toBeNull();
});

test("an_even_count_is_unequal_when_the_hours_are", () => {
  const page = show(HelpedBars, {
    helped: { ...base, before: 6, after: 6, before_ms: 100 * HOUR, after_ms: 20 * HOUR, verdict: "More" },
    metric: "stutters",
  });

  const [before, after] = widths(page);
  expect(after).toBeGreaterThan(before);
  expect(page.textContent).toContain("More since the change");
});

test("too_few_events_draw_outlines_with_the_counts_and_say_how_many_are_needed", () => {
  const page = show(HelpedBars, {
    helped: { ...base, before: 2, after: 1, verdict: "NotEnoughEvents" },
    metric: "stutters",
  });

  expect(page.querySelector(".helped")?.classList.contains("lacking")).toBe(true);
  expect(page.textContent).toContain("2 events");
  expect(page.textContent).toContain("Not enough data yet");
  expect(page.textContent).toContain("Only 3 events in both periods; the comparison starts at 5.");
});

test("a_side_watched_too_briefly_is_tagged_and_explained", () => {
  const page = show(HelpedBars, {
    helped: { ...base, before: 12, after: 2, after_ms: 2 * HOUR, verdict: "NotEnoughTime" },
    metric: "stutters",
  });

  expect(page.textContent).toContain("only 2 h");
  expect(page.textContent).not.toContain("only 168 h");
  expect(page.textContent).toContain(
    "The PC was monitored for only 2 h after the change; the comparison needs 12 h on each side.",
  );
});

test("nothing_monitored_draws_without_dividing_by_zero", () => {
  const page = show(HelpedBars, {
    helped: { ...base, before_ms: 0, after_ms: 0, verdict: "NotEnoughTime" },
    metric: "stutters",
  });

  expect(widths(page).every((w) => Number.isFinite(w))).toBe(true);
  expect(page.textContent).toContain("only 0 h before and 0 h after");
});

test("the_rate_is_written_with_the_languages_decimal_mark", () => {
  setLang("pt-BR");
  const page = show(HelpedBars, { helped: base, metric: "engasgos" });

  expect(page.textContent).toContain("0,11 por hora");
  expect(page.textContent).toContain("18 em 168 h");
  expect(page.textContent).toContain("Menos desde a alteração");
});
