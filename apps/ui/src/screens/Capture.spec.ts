import { expect, test } from "vitest";

import { capture } from "../dev/fixtures/capture";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { t } from "../lib/i18n.svelte";
import type { CaptureDto } from "../lib/types";
import Capture from "./Capture.svelte";
import { rowOf } from "./capture/words";

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

  const rows = [...page.querySelectorAll("li")].filter((li) => words(li).includes("Starfall"));
  expect(rows.length).toBeGreaterThan(0);
  expect(words(rows[0])).toContain(t("ui.captures.apps.over", { limit: 5 }));
  expect(words(rows[0])).toContain("8.0 %");
  expect(words(page)).toContain("is not named as a suspect");
  const steady = [...page.querySelectorAll("li")].find((li) => words(li).includes("Editor"));
  expect(steady).toBeUndefined();
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
