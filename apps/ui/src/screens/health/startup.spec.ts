import { expect, test } from "vitest";

import { setLang, t } from "../../lib/i18n.svelte";
import type { BootDto } from "../../lib/types";
import {
  appsMs,
  bootKind,
  ceilingOf,
  failures,
  formatSeconds,
  judge,
  ledeParts,
  neighbours,
  niceCeiling,
  phaseMs,
  profileAfterTpmMs,
  series,
  startupWhen,
  valueText,
  waterfall,
  waterfallCeiling,
} from "./startup";

const KERNEL = 1_800_000_000_000;

/** A measured startup with every phase present; a test overrides what it is about. */
function boot(over: Partial<BootDto> = {}): BootDto {
  return {
    kernel_start_ms: KERNEL,
    boot_type: 0,
    firmware_ms: 14_000,
    to_signin_ms: 20_000,
    probable_wait: false,
    profile_ms: 8_000,
    tpm_wait_ms: null,
    desktop_ms: 8_000,
    to_desktop_ms: 36_000,
    from_power_ms: 50_000,
    apps: [],
    logger_full: false,
    failed_drivers: [],
    ...over,
  };
}

test("a_duration_is_seconds_with_one_decimal_below_two_minutes_and_minutes_after", () => {
  expect(formatSeconds(36_300)).toBe("36.3 s");
  expect(formatSeconds(36_000)).toBe("36 s");
  expect(formatSeconds(152_000)).toBe("2 min 32 s");
  expect(formatSeconds(120_000)).toBe("2 min");
  expect(formatSeconds(119_700)).toBe("2 min");
  expect(formatSeconds(108_200)).toBe("108 s");
});

test("a_duration_uses_the_decimal_separator_of_the_language", () => {
  setLang("pt-BR");
  expect(formatSeconds(36_300)).toBe("36,3 s");
  expect(formatSeconds(152_000)).toBe("2 min 32 s");
});

test("a_phase_that_was_not_measured_says_so_and_is_never_zero", () => {
  expect(valueText(null)).toBe(t("ui.health.startup.value.unmeasured"));
  expect(valueText(null)).not.toMatch(/0/);
  const gone = boot({ firmware_ms: null, desktop_ms: null, profile_ms: null, to_signin_ms: null });
  for (const id of ["firmware", "signin", "profile", "desktop", "apps"] as const) expect(phaseMs(gone, id)).toBeNull();
  expect(series([gone], "profile")).toEqual([null]);
});

test("the_startup_apps_phase_runs_from_the_desktop_to_the_first_app", () => {
  const apps = [
    { command: String.raw`C:\Example\later.exe`, start_ms: KERNEL + 48_000, end_ms: null },
    { command: String.raw`C:\Example\first.exe`, start_ms: KERNEL + 46_900, end_ms: KERNEL + 49_000 },
  ];

  expect(appsMs(boot({ apps }))).toBe(10_900);
});

test("the_startup_apps_phase_is_not_measured_without_apps_or_a_desktop_time", () => {
  const run = { command: "app.exe", start_ms: KERNEL + 46_900, end_ms: null };

  expect(appsMs(boot({ apps: [] }))).toBeNull();
  expect(appsMs(boot({ apps: [run], to_desktop_ms: null }))).toBeNull();
  expect(appsMs(boot({ apps: [{ ...run, start_ms: KERNEL + 30_000 }] }))).toBeNull();
});

test("only_the_profile_phase_above_sixty_seconds_is_an_alert", () => {
  expect(judge(boot({ profile_ms: 61_000 }), "profile").kind).toBe("attention");
  expect(judge(boot({ profile_ms: 31_000 }), "profile")).toEqual({ kind: "info", word: t("ui.health.startup.word.above") });
  expect(judge(boot({ profile_ms: 30_000 }), "profile").kind).toBe("ok");
  expect(judge(boot({ firmware_ms: 26_000 }), "firmware").kind).toBe("info");
  expect(judge(boot({ desktop_ms: 11_000 }), "desktop").kind).toBe("info");
});

test("a_probable_wait_is_information_and_a_missing_phase_is_not_measured", () => {
  expect(judge(boot({ to_signin_ms: 708_000, probable_wait: true }), "signin")).toEqual({
    kind: "info",
    word: t("ui.health.startup.word.wait"),
  });
  expect(judge(boot({ firmware_ms: null }), "firmware")).toEqual({ kind: "info", word: t("ui.health.startup.word.unmeasured") });
  expect(judge(boot(), "apps").kind).toBe("info");
  expect(judge(boot({ apps: [{ command: "a.exe", start_ms: KERNEL + 46_900, end_ms: null }] }), "apps").kind).toBe("ok");
});

test("a_scale_reaches_the_limit_and_every_startup_shown", () => {
  expect(niceCeiling(361)).toBe(400);
  expect(ceilingOf([boot({ profile_ms: 8_000 })], "profile")).toBe(30);
  expect(ceilingOf([boot({ profile_ms: 361_000 })], "profile")).toBe(400);
  expect(ceilingOf([boot()], "apps")).toBe(5);
});

test("the_lede_counts_from_the_button_then_from_windows_then_says_nothing_was_measured", () => {
  const text = (b: BootDto) => ledeParts([b]).map((p) => p.text).join("");

  expect(ledeParts([boot()]).find((p) => p.bold)?.text).toBe("50 s");
  expect(text(boot())).toContain(t("ui.health.startup.lede.power", { when: "" }).slice(0, 20));
  expect(text(boot({ from_power_ms: null }))).toContain(t("ui.health.startup.lede.kernel", { when: "" }).slice(0, 20));
  const none = text(boot({ from_power_ms: null, to_desktop_ms: null }));
  expect(none).toContain(t("ui.health.startup.lede.unmeasured", { when: "" }).slice(0, 30));
  expect(ledeParts([boot({ from_power_ms: null, to_desktop_ms: null })]).some((p) => p.bold)).toBe(false);
  expect(text(boot({ probable_wait: true }))).toContain(t("ui.health.startup.lede.wait"));
  expect(ledeParts([])[0]).toEqual({ text: t("ui.health.startup.lede.none"), bold: true });
});

test("failed_drivers_count_the_startups_they_failed_in_once_each", () => {
  const boots = [
    boot({ failed_drivers: ["dam", "dam", "otherdrv"] }),
    boot({ failed_drivers: [] }),
    boot({ failed_drivers: ["dam"] }),
  ];

  expect(failures(boots)).toEqual([
    { name: "dam", count: 2 },
    { name: "otherdrv", count: 1 },
  ]);
});

const segment = (boot: BootDto, id: string) => waterfall(boot).segments.find((s) => s.id === id)!;

test("the_waterfall_places_each_phase_after_the_one_before_it_from_the_power_button", () => {
  const w = waterfall(boot());

  expect(w.fromWindows).toBe(false);
  expect(w.segments.map((s) => [s.id, s.from, s.to])).toEqual([
    ["firmware", 0, 14],
    ["signin", 14, 34],
    ["profile", 34, 42],
    ["desktop", 42, 50],
    ["apps", null, null],
  ]);
  expect(w.tpmAt).toBeNull();
  expect(w.end).toBe(50);
});

test("the_waterfall_puts_the_startup_apps_from_the_desktop_to_the_first_app_and_the_tpm_line_in_the_profile", () => {
  const run = { command: "app.exe", start_ms: KERNEL + 46_900, end_ms: null };
  const w = waterfall(boot({ apps: [run], tpm_wait_ms: 5_000 }));

  expect(w.segments[4]).toEqual({ id: "apps", from: 50, to: 60.9 });
  expect(w.tpmAt).toBe(39);
  expect(w.end).toBe(60.9);
});

test("the_waterfall_counts_from_the_start_of_windows_when_the_firmware_time_is_missing", () => {
  const w = waterfall(boot({ firmware_ms: null, from_power_ms: null }));

  expect(w.fromWindows).toBe(true);
  expect(w.segments[0]).toEqual({ id: "firmware", from: null, to: null });
  expect(segment(boot({ firmware_ms: null }), "signin")).toEqual({ id: "signin", from: 0, to: 20 });
  expect(w.segments[3].to).toBe(36);
});

test("a_phase_that_was_not_measured_is_not_placed_and_has_no_length", () => {
  const w = waterfall(boot({ desktop_ms: null, to_desktop_ms: null, profile_ms: null }));

  expect(w.segments.find((s) => s.id === "profile")).toEqual({ id: "profile", from: null, to: null });
  expect(w.segments.find((s) => s.id === "desktop")).toEqual({ id: "desktop", from: null, to: null });
  expect(w.tpmAt).toBeNull();
  const nothing = waterfall(boot({ firmware_ms: null, to_signin_ms: null, profile_ms: null, desktop_ms: null, to_desktop_ms: null }));
  expect(nothing.segments.every((s) => s.from === null && s.to === null)).toBe(true);
  expect(nothing.end).toBe(0);
});

test("a_tpm_wait_of_zero_draws_no_tpm_line", () => {
  expect(waterfall(boot({ tpm_wait_ms: 0 })).tpmAt).toBeNull();
});

test("the_waterfall_axis_ends_at_a_round_number_past_the_last_phase", () => {
  expect(waterfallCeiling(waterfall(boot()))).toBe(60);
  expect(waterfallCeiling(waterfall(boot({ to_desktop_ms: 173_000, desktop_ms: 145_000, profile_ms: 8_000 })))).toBe(200);
});

test("the_profile_after_the_tpm_is_what_is_left_of_the_profile_step", () => {
  expect(profileAfterTpmMs(boot({ profile_ms: 152_000, tpm_wait_ms: 37_000 }))).toBe(115_000);
  expect(profileAfterTpmMs(boot({ tpm_wait_ms: 0 }))).toBeNull();
  expect(profileAfterTpmMs(boot({ tpm_wait_ms: null }))).toBeNull();
});

test("the_kind_of_startup_comes_from_the_recorded_type_and_is_empty_for_any_other", () => {
  expect([0, 1, 2, 7, null].map((boot_type) => bootKind(boot({ boot_type })))).toEqual(["full", "fast", "resume", null, null]);
});

test("the_neighbours_of_a_startup_are_the_ones_next_to_it_in_the_list", () => {
  const list = [0, 1, 2].map((n) => boot({ kernel_start_ms: KERNEL + n }));

  expect(neighbours(list, KERNEL + 1)).toEqual({ boot: list[1], prev: list[0], next: list[2] });
  expect(neighbours(list, KERNEL)).toMatchObject({ prev: null, next: list[1] });
  expect(neighbours(list, KERNEL + 2)).toMatchObject({ prev: list[1], next: null });
  expect(neighbours(list, 5)).toEqual({ boot: null, prev: null, next: null });
});

test("a_startup_is_dated_with_the_day_and_the_time_never_today", () => {
  const text = startupWhen(Date.now());

  expect(text).not.toContain(t("ui.health.today_at", { time: "" }).trim());
  expect(text).toMatch(/\d{1,2}:\d{2}/);
});
