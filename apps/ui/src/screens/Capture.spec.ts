import { expect, test } from "vitest";

import { capture } from "../dev/fixtures/capture";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { t } from "../lib/i18n.svelte";
import type { CaptureDto } from "../lib/types";
import Capture from "./Capture.svelte";
import { buildResult, resultMarkdown, rowOf } from "./capture/words";

const START = Date.UTC(2026, 9, 20, 12, 0);

/** Opens the page of a stored capture and waits until it has drawn. */
async function open(dto: CaptureDto): Promise<HTMLElement> {
  answer({ capture: () => dto, marks: () => ({ occurrences: [] }) });
  const page = show(Capture, { id: dto.id });
  await until(() => page.querySelector("h1") !== null && page.querySelector(".facts") !== null, "the capture page");
  return page;
}

const words = (el: Element | null) => el?.textContent?.replace(/\s+/g, " ").trim() ?? "";

test("an_app_with_eight_percent_late_frames_stays_in_the_list_with_a_mark_and_is_not_a_suspect", async () => {
  const page = await open(capture(5, START));

  const rows = [...page.querySelectorAll(".card")].filter((card) => words(card).includes("Starfall"));
  expect(rows.length).toBeGreaterThan(0);
  expect(words(rows[0])).toContain(t("ui.captures.apps.over", { limit: 5 }));
  expect(words(rows[0])).toContain("8.0 %");
  expect(words(page)).toContain("is not named as a suspect");
  expect([...page.querySelectorAll(".card")].some((card) => words(card).includes("Editor"))).toBe(false);
  expect(page.querySelector(".cz-n")?.textContent).not.toContain("Starfall");
});

const PARTS = { Drivers: "kernel", Frames: "frames", Stalls: "probe" } as const;
const sourceRow = (page: HTMLElement, name: keyof typeof PARTS) => page.querySelector(`.cov tr[data-part=${PARTS[name]}]`);

test("a_capture_whose_frames_stopped_after_three_of_five_minutes_says_so_and_still_names_its_driver", async () => {
  const page = await open(capture(6, START));

  expect(words(sourceRow(page, "Frames")!)).toContain("3 of 5 min");
  expect(words(sourceRow(page, "Drivers")!)).toContain("5 of 5 min");
  expect(words(sourceRow(page, "Stalls")!)).toContain("5 of 5 min");
  expect(words(page.querySelector(".facts"))).toContain("Partial");
  expect(page.querySelector(".cz-n")?.textContent).toContain("gfxdriver.sys");
  expect(page.querySelector(".cov .bar i")).not.toBeNull();
});

test("a_capture_that_could_not_measure_drivers_says_which_source_is_missing_and_what_to_close", async () => {
  const page = await open(capture(9, START));

  expect(words(sourceRow(page, "Drivers")!)).toContain("Not measured");
  expect(words(sourceRow(page, "Drivers")!)).toContain("—");
  expect(words(page.querySelector(".facts"))).toContain("drivers missing");
  expect(words(page)).toContain("LatencyMon");
});

test("a_capture_with_every_source_measured_is_complete", async () => {
  const page = await open(capture(4, START));

  expect(words(page.querySelector(".facts"))).toContain("Complete");
  expect(words(sourceRow(page, "Frames")!)).toContain("5 of 5 min");
});

test("what_is_kept_counts_the_stored_capture_not_the_page_that_trims_it", async () => {
  const dto = { ...capture(1, START), kept: { calls: 40, hitches: 12 } };
  const page = await open(dto);

  expect(words(sourceRow(page, "Drivers")!)).toContain("40 long calls, the worst 3 per minute");
  expect(words(sourceRow(page, "Frames")!)).toContain("12 slow frames");
  expect(words(sourceRow(page, "Stalls")!)).toContain("3 stalls");
});

test("a_history_row_carries_the_coverage_mark_and_says_how_far_the_frames_got", () => {
  const entry = (dto: CaptureDto) => ({ dto, summary: { id: dto.id, start_ms: dto.start_ms, end_ms: dto.end_ms } });

  const partial = rowOf(entry(capture(6, START)));
  const whole = rowOf(entry(capture(4, START)));

  expect(partial.cover.word).toBe(t("ui.captures.cover.partial"));
  expect(partial.sub).toContain("frames 3 of 5 min");
  expect(whole.cover.word).toBe(t("ui.captures.cover.full"));
  expect(whole.sub).not.toContain("of 5 min");
});

const entryOf = (dto: CaptureDto) => ({ dto, summary: { id: dto.id, start_ms: dto.start_ms, end_ms: dto.end_ms } });
const NO_STUTTERS = "ui.captures.title.none";

test("stalls_measured_with_no_driver_over_the_limits_say_no_culprit_not_no_stutters", () => {
  const dto = { ...capture(8, START), findings: [] };

  const view = buildResult(dto, "Simple");

  expect(view.verdict).toBe("nocul");
  expect(view.title).toBe(t("ui.captures.title.nocul"));
  expect(view.title).not.toBe(t(NO_STUTTERS));
  expect(view.otherwise?.body).toContain("3 stalls were measured");
  expect(view.otherwise?.offer).toBe(false);
  expect(view.result.word).toBe(t("ui.captures.mark.nocul"));
  expect(rowOf(entryOf(dto)).title).toBe(t("ui.captures.title.nocul"));
  expect(rowOf(entryOf(dto)).sub).toContain("3 stalls measured");
});

test("an_app_with_many_late_frames_and_no_culprit_is_a_no_culprit_capture_that_points_at_the_app", () => {
  const view = buildResult(capture(5, START), "Simple");

  expect(view.verdict).toBe("nocul");
  expect(view.otherwise?.body).toContain("Starfall has many slow frames (8.0 %)");
  expect(view.steps.map((s) => s.text).join(" ")).toContain("If Starfall really stutters");
  expect(rowOf(entryOf(capture(5, START))).sub).toContain("Starfall with 8.0 % slow frames");
});

test("a_capture_whose_frames_stopped_early_and_found_nothing_says_it_cannot_tell", () => {
  const dto = capture(7, START);

  const view = buildResult(dto, "Simple");

  expect(view.verdict).toBe("incomplete");
  expect(view.title).toBe(t("ui.captures.title.undetermined"));
  expect(view.otherwise?.head).toBe(t("ui.captures.incomplete.head"));
  expect(view.otherwise?.body).toContain("first 3 of 5 minutes");
  expect(view.otherwise?.offer).toBe(true);
  expect(view.result.word).toBe(t("ui.captures.mark.undetermined"));
  expect(rowOf(entryOf(dto)).title).toBe(t("ui.captures.title.undetermined"));
});

test("a_capture_that_measured_no_driver_cannot_say_there_were_no_stutters_even_when_the_trace_reported_ok", () => {
  const failed = capture(9, START);
  const empty: CaptureDto = { ...capture(4, START), kernel: "Ok", drivers: [] };

  expect(buildResult(failed, "Simple").verdict).toBe("incomplete");
  expect(buildResult(failed, "Simple").otherwise?.sub).toContain("frames");
  expect(buildResult(empty, "Simple").verdict).toBe("incomplete");
  expect(buildResult(empty, "Simple").title).toBe(t("ui.captures.title.undetermined"));
});

test("a_partial_capture_that_names_a_driver_keeps_naming_it", () => {
  const view = buildResult(capture(6, START), "Simple");

  expect(view.verdict).toBe("found");
  expect(view.title).toContain("gfxdriver.sys");
  expect(view.otherwise).toBeNull();
  expect(view.cover.mark.word).toBe(t("ui.captures.cover.partial"));
});

test("a_capture_that_measured_everything_and_saw_nothing_keeps_the_no_stutters_wording", () => {
  const view = buildResult(capture(4, START), "Simple");

  expect(view.verdict).toBe("clean");
  expect(view.title).toBe(t(NO_STUTTERS));
  expect(view.otherwise?.head).toBe(t("ui.captures.none.head"));
  expect(view.otherwise?.offer).toBe(true);
  expect(view.result.word).toBe(t("ui.captures.mark.quiet"));
  expect(rowOf(entryOf(capture(4, START))).title).toBe(t("ui.captures.row.none.title"));
});

test("the_three_cases_show_three_different_titles_and_history_rows", () => {
  const cases = [capture(8, START), capture(7, START), capture(4, START)];

  expect(new Set(cases.map((c) => buildResult(c, "Simple").title)).size).toBe(3);
  expect(new Set(cases.map((c) => rowOf(entryOf(c)).title)).size).toBe(3);
});

test("a_source_that_fell_short_adds_its_own_step_and_the_copied_result_lists_what_was_measured", () => {
  const view = buildResult(capture(9, START), "Simple");

  expect(view.steps.map((s) => s.text).join(" ")).toContain("LatencyMon");
  const md = resultMarkdown(capture(6, START));
  expect(md).toContain(`### ${t("ui.captures.md.cover")}`);
  expect(md).toContain("Frames: 3 of 5 min");
});

test("the_page_of_an_incomplete_capture_offers_a_longer_one", async () => {
  const page = await open(capture(7, START));

  expect(page.querySelector("h1")?.textContent).toContain(t("ui.captures.title.undetermined"));
  expect(words(page.querySelector(".cz-n"))).toBe(t("ui.captures.incomplete.head"));
  expect(words(page.querySelector(".acts"))).toContain(t("ui.captures.none.later"));
});
