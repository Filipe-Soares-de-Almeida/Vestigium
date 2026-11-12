import { afterEach, beforeEach, expect, test } from "vitest";

import { testsDtoSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { t } from "../lib/i18n.svelte";
import { hwtests } from "../lib/tests.svelte";
import type { DiskDetailDto, Media, SmartAttribute, TestRunDto, TestsDto, VolumeCheck, VolumeResult } from "../lib/types";
import Tests from "./Tests.svelte";
import { diskState, nvmeMarks, repairMark, smartWarns } from "./tests/words";

const NOW = Date.now();
const HOUR = 3_600_000;
const BOOT = NOW - 10 * HOUR;

/** A volume check of C: that ended `ago` ms before now; the dirty bits are what the helper stored. */
function check(id: number, ago: number, result: VolumeResult, before: boolean | null, after: boolean | null): TestRunDto {
  const data: VolumeCheck = { volume: "C:\\", exit_code: null, result, output: "", dirty_before: before, dirty_after: after };
  return {
    id,
    started_ms: NOW - ago - 60_000,
    ended_ms: NOW - ago,
    subject: { Volume: "C:\\" },
    outcome: result === "ProblemsFound" ? "Problems" : result === "NoProblems" ? "Clean" : "Failed",
    data: { VolumeCheck: data },
  };
}

/** The Tests page with one NTFS volume and the given runs, oldest first. */
function page(runs: TestRunDto[], bootMs = BOOT): TestsDto {
  const base = testsDtoSamples[1];
  return {
    ...base,
    disks: [{ ...base.disks[0], number: 0, name: "0 C:", volumes: ["C:\\"] }],
    volumes: [{ root: "C:\\", total_bytes: 10 ** 12, free_bytes: 5 * 10 ** 11, system: true, disk: 0, file_system: "NTFS" }],
    runs,
    running: null,
    scan: null,
    boot_ms: bootMs,
  };
}

beforeEach(() => {
  hwtests.dto = null;
  hwtests.running = null;
});

afterEach(() => {
  hwtests.dto = null;
});

async function open(dto: TestsDto): Promise<HTMLElement> {
  answer({ tests: () => dto, health: () => Promise.reject(new Error("unused")), disk_detail: () => ({ health: null, nvme: null }) });
  const el = show(Tests, { device: "0" });
  await until(() => el.querySelector(".vol") !== null, "the volume card");
  return el;
}

const panelOf = (el: HTMLElement) => el.querySelector<HTMLElement>(".vol .panel");

test("a_check_that_marked_the_volume_shows_the_repair_panel_with_its_reminder_to_back_up", async () => {
  const el = await open(page([check(1, HOUR, "ProblemsFound", false, true)]));

  const panel = panelOf(el);
  expect(panel).not.toBeNull();
  expect(panel!.textContent).toContain(t("ui.tests.check.panel.new", { volume: "C:" }));
  expect(panel!.textContent).toContain(t("ui.tests.check.panel.body"));
});

test("a_volume_already_marked_before_the_check_says_so_in_the_panel", async () => {
  const el = await open(page([check(1, HOUR, "ProblemsFound", true, true)]));

  expect(panelOf(el)!.textContent).toContain(t("ui.tests.check.panel.already", { volume: "C:" }));
});

test("a_clean_check_of_a_volume_still_marked_keeps_the_panel_and_says_still", async () => {
  const el = await open(page([check(1, 2 * HOUR, "ProblemsFound", false, true), check(2, HOUR, "NoProblems", true, true)]));

  expect(panelOf(el)!.textContent).toContain(t("ui.tests.check.panel.still", { volume: "C:" }));
});

test("a_clean_volume_card_has_no_panel", async () => {
  const el = await open(page([check(1, HOUR, "NoProblems", false, false)]));

  expect(panelOf(el)).toBeNull();
  expect(el.querySelector(".vol")!.textContent).toContain(t("ui.tests.check.NoProblems"));
});

test("a_check_the_window_never_ran_leaves_no_panel", async () => {
  const el = await open(page([]));

  expect(panelOf(el)).toBeNull();
});

test("the_panel_goes_once_a_boot_is_newer_than_the_check", async () => {
  const el = await open(page([check(1, 11 * HOUR, "ProblemsFound", false, true)]));

  expect(panelOf(el)).toBeNull();
});

test("a_newer_check_that_left_the_volume_unmarked_ends_the_panel", () => {
  const dto = page([check(1, 2 * HOUR, "ProblemsFound", false, true), check(2, HOUR, "NoProblems", false, false)]);

  expect(repairMark(dto, "C:\\")).toBeNull();
});

test("a_check_that_could_not_read_the_volume_keeps_the_mark_of_the_one_before", () => {
  const dto = page([check(1, 2 * HOUR, "ProblemsFound", false, true), check(2, HOUR, "CouldNotCheck", null, null)]);

  expect(repairMark(dto, "C:\\")).toBe("new");
});

test("a_problems_result_stored_before_the_dirty_bits_existed_counts_as_marked", () => {
  const dto = page([check(1, HOUR, "ProblemsFound", null, null)]);

  expect(repairMark(dto, "C:\\")).toBe("new");
});

test("another_volumes_check_does_not_mark_this_one", () => {
  const dto = page([check(1, HOUR, "ProblemsFound", false, true)]);

  expect(repairMark(dto, "D:\\")).toBeNull();
});

const attr = (id: number, raw: number): SmartAttribute => ({ id, current: 100, worst: 100, raw });

/** The disk state of disk 0 as `media` with the given SMART attributes and no test runs. */
function stateWith(media: Media, attributes: SmartAttribute[]) {
  const dto = page([]);
  const disk = { ...dto.disks[0], media };
  const detail: DiskDetailDto = { health: { name: disk.name, read_ms: NOW, predict_failure: false, attributes, nvme: null }, nvme: null };
  return diskState(dto, disk, detail, false).mark;
}

test("a_watched_counter_above_zero_warns_on_a_hard_drive_only", () => {
  expect(smartWarns(attr(187, 1), "Hdd")).toBe(true);
  expect(smartWarns(attr(187, 1), "Ssd")).toBe(false);
  expect(smartWarns(attr(187, 1), "Unknown")).toBe(false);
  expect(smartWarns(attr(9, 4000), "Hdd")).toBe(false);
  expect(smartWarns(attr(5, 0), "Hdd")).toBe(false);
});

test("attribute_188_warns_by_its_low_16_bits", () => {
  expect(smartWarns(attr(188, 0x1_0000_0000), "Hdd")).toBe(false);
  expect(smartWarns(attr(188, 0x1_0000_0002), "Hdd")).toBe(true);
});

test("an_ssd_with_a_watched_counter_above_zero_is_not_marked_attention", () => {
  expect(stateWith("Ssd", [attr(187, 1)])).not.toBe("Attention");
  expect(stateWith("Hdd", [attr(187, 1)])).toBe("Attention");
});

test("nvme_rows_take_the_levels_of_the_health_findings", () => {
  const healthy = { critical_warning: 0, temperature_c: 40, available_spare_pct: 100, spare_threshold_pct: 10, percentage_used: 3, media_errors: 0 };
  expect(nvmeMarks(healthy)).toEqual({ used: null, spare: null, media: null });
  expect(nvmeMarks({ ...healthy, percentage_used: 90, available_spare_pct: 5, media_errors: 2 })).toEqual({
    used: "attention",
    spare: "critical",
    media: "attention",
  });
});
