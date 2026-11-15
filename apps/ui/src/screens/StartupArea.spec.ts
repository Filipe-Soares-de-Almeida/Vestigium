import { flushSync } from "svelte";
import { afterEach, beforeEach, expect, test } from "vitest";

import { startupCase, type Case } from "../dev/fixtures/startup";
import { healthDtoSamples } from "../dev/fixtures/wire";
import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { health } from "../lib/health.svelte";
import { errorText } from "../lib/errors";
import { t } from "../lib/i18n.svelte";
import { href } from "../lib/router.svelte";
import { toast } from "../lib/toast.svelte";
import type { DetailLevel, FindingDto, HealthDto } from "../lib/types";
import Health from "./Health.svelte";
import StartupArea from "./health/StartupArea.svelte";

const k = (key: string, args?: Record<string, string | number>) => t(`ui.health.startup.${key}`, args);

/** Mounts the area with the data of a case and waits for its first sentence. */
async function open(which: Case, props: { finding?: FindingDto | null; level?: DetailLevel } = {}) {
  const calls = answer({ startup_area: () => startupCase(which), open_storage_sense: () => null });
  const page = show(StartupArea, { finding: props.finding ?? null, level: props.level ?? "Simple" });
  await until(() => page.querySelector(".lede") !== null, "the lede");
  return { page, calls };
}

/** The row of a list whose text starts with `title`. */
function row(page: HTMLElement, title: string): HTMLElement {
  const found = [...page.querySelectorAll<HTMLElement>("li")].find((li) => li.querySelector(".title")?.textContent === title);
  if (!found) throw new Error(`no row ${title}`);
  return found;
}

const phaseRow = (page: HTMLElement, id: string) => row(page, k(`phase.${id}`));
function click(el: Element | null): void {
  (el as HTMLElement).click();
  flushSync();
}

/** The kind of the status mark of a row: ok, info, attention. */
function kindOf(li: HTMLElement): string | undefined {
  return [...(li.querySelector(".status")?.classList ?? [])].find((c) => ["ok", "info", "attention"].includes(c));
}

afterEach(() => {
  toast.on = false;
});

test("the_slow_startup_puts_the_profile_phase_in_attention_and_calls_the_temp_folder_large", async () => {
  const { page } = await open("slow");

  expect(kindOf(phaseRow(page, "profile"))).toBe("attention");
  expect(phaseRow(page, "profile").textContent).toContain(t("ui.health.mark.Attention"));
  const temp = row(page, k("temp.title"));
  expect(temp.textContent).toContain(k("temp.large"));
  expect(temp.textContent).toContain("213,879");
  expect(page.querySelector(".lede")?.textContent).toContain("from the power button");
});

test("a_fast_startup_has_every_phase_within_normal_and_a_lede_in_seconds", async () => {
  const { page } = await open("fast");

  for (const id of ["firmware", "signin", "profile", "desktop", "apps"]) expect(kindOf(phaseRow(page, id))).toBe("ok");
  expect(page.querySelector(".lede b")?.textContent).toBe("49.8 s");
  expect(page.querySelector(".lede")?.textContent).not.toContain(k("lede.wait"));
  expect(row(page, k("temp.title")).textContent).not.toContain(k("temp.large"));
  expect(phaseRow(page, "apps").textContent).toContain("10.6 s");
});

test("a_wait_on_the_sign_in_screen_is_information_and_the_lede_says_so", async () => {
  const { page } = await open("wait");

  const signin = phaseRow(page, "signin");
  expect(kindOf(signin)).toBe("info");
  expect(signin.textContent).toContain(k("word.wait"));
  expect(page.querySelector(".status.critical")).toBeNull();
  expect(page.querySelector(".lede")?.textContent).toContain(k("lede.wait"));
  expect(page.querySelector(".lede b")?.textContent).toBe("14 min 19 s");
});

test("with_no_startup_measured_every_phase_is_not_measured_and_nothing_is_called_normal", async () => {
  const { page } = await open("none");

  expect(page.querySelector(".lede")?.textContent).toContain(k("lede.none"));
  for (const id of ["firmware", "signin", "profile", "desktop", "apps"]) {
    const li = phaseRow(page, id);
    expect(li.textContent).toContain(k("value.unmeasured"));
    expect(li.textContent).toContain(t("ui.health.mark.Unknown"));
    expect(li.querySelector("button")).toBeNull();
  }
  const text = page.textContent ?? "";
  expect(text).not.toMatch(/\b0 s\b/);
  expect(text).not.toContain(k("body.ok"));
  expect(text).not.toContain(t("ui.health.mark.Ok"));
  expect(page.querySelector(".plot")).toBeNull();
  expect(row(page, "ChatClient")).not.toBeNull();
  expect(row(page, k("temp.title")).textContent).toContain(k("temp.unmeasured"));
});

test("the_profile_row_opens_to_the_tpm_wait_and_a_chart_with_the_zoom_bar", async () => {
  const { page } = await open("slow");
  const profile = phaseRow(page, "profile");

  click(profile.querySelector("button"));
  await until(() => profile.querySelector(".zoombar") !== null, "the zoom bar");

  expect(profile.querySelector("button")?.getAttribute("aria-expanded")).toBe("true");
  expect(profile.textContent).toContain(k("body.tpm_waited", { time: "37 s" }));
  expect(profile.querySelector(".plot")?.getAttribute("aria-label")).toBe(k("phase.profile"));
  expect(profile.textContent).not.toContain(k("source", { name: "User Profile Service" }));
});

test("the_technical_level_names_the_event_a_phase_comes_from", async () => {
  const { page } = await open("slow", { level: "Technical" });

  click(phaseRow(page, "firmware").querySelector("button"));

  expect(phaseRow(page, "firmware").textContent).toContain(k("source", { name: "Kernel-Boot 30" }));
});

test("an_open_slow_startup_finding_shows_inside_the_profile_row_without_its_title", async () => {
  const finding: FindingDto = { ...healthDtoSamples[0].open[0], kind: "SlowBoot", subject: "Pc" };
  const { page } = await open("slow", { finding });

  click(phaseRow(page, "profile").querySelector("button"));

  const inside = phaseRow(page, "profile");
  expect(inside.querySelector("section.finding")).not.toBeNull();
  expect(inside.querySelector("h2")).toBeNull();
});

test("the_apps_row_lists_the_programs_the_latest_startup_opened", async () => {
  const { page } = await open("fast");

  click(phaseRow(page, "apps").querySelector("button"));

  const text = phaseRow(page, "apps").textContent ?? "";
  expect(text).toContain("vestigium.exe");
  expect(text).toContain("capture.exe");
  expect(text).toContain(k("run.opened", { at: "46.9 s" }));
  expect(text).toContain(k("run.took", { took: "2.8 s" }));
});

test("only_an_enabled_app_that_is_not_security_software_or_vestigium_offers_disable", async () => {
  const { page } = await open("slow");

  const chat = row(page, "ChatClient").querySelector("a");
  expect(chat?.getAttribute("href")).toBe(href.tweaks("StartupApp:ChatClient"));
  expect(row(page, "SecurityTray").querySelector("a")).toBeNull();
  expect(row(page, "SecurityTray").textContent).toContain(k("apps.protected"));
  expect(row(page, "Example Capture").querySelector("a")).toBeNull();
  expect(row(page, "Example Capture").textContent).toContain(k("apps.off"));
  expect(row(page, "Vestigium").querySelector("a")?.getAttribute("href")).toBe(href.settings("watching"));
  expect(row(page, "Vestigium").textContent).not.toContain(k("apps.disable"));
  expect(page.textContent).toContain(k("apps.note"));
});

test("a_driver_that_failed_shows_in_how_many_startups_and_the_known_one_explains_itself", async () => {
  const { page } = await open("slow");
  const total = startupCase("slow").boots.length;

  const dam = row(page, k("fail.row", { name: "dam" }));
  expect(dam.textContent).toContain(k("fail.sub.other", { count: 5, total }));
  expect(row(page, k("fail.row", { name: "exampledrv" })).querySelector("button")).toBeNull();
  click(dam.querySelector("button"));
  expect(dam.textContent).toContain(k("fail.dam").slice(0, 30));
});

test("the_temp_row_opens_storage_sense_and_a_failed_open_shows_a_toast", async () => {
  const { page, calls } = await open("slow");
  const temp = row(page, k("temp.title"));

  click(temp.querySelector("button"));
  click(temp.querySelector(".acts button"));
  await until(() => calls.of("open_storage_sense").length === 1, "the command");
  expect(toast.on).toBe(false);

  answer({ startup_area: () => startupCase("slow"), open_storage_sense: () => Promise.reject(refusal("failed")) });
  click(temp.querySelector(".acts button"));
  await until(() => toast.on, "the toast");
  expect(toast.message).toBe(errorText(refusal("failed")));
});

test("the_startups_list_holds_every_startup_newest_first_in_its_own_scrolling_box", async () => {
  const { page } = await open("slow");
  const boots = startupCase("slow").boots;
  const list = row(page, t("ui.health.startup.boots.title.other", { count: boots.length }));

  click(list.querySelector("button"));

  expect(list.querySelector(".scroll")).not.toBeNull();
  const items = list.querySelectorAll("ol.hist li");
  expect(items).toHaveLength(boots.length);
  expect(items[0].textContent).toContain(k("boots.profile", { time: "2 min 32 s" }));
  expect(items[0].querySelector("a")).toBeNull();
});

test("a_failed_read_shows_the_error_and_try_again_loads_the_page", async () => {
  let failing = true;
  const calls = answer({ startup_area: () => (failing ? Promise.reject(refusal("store")) : startupCase("fast")) });
  const page = show(StartupArea, { finding: null, level: "Simple" });
  await until(() => page.querySelector(".failure") !== null, "the error");

  failing = false;
  click(page.querySelector(".failure .btn"));
  await until(() => page.querySelector(".lede") !== null, "the lede");

  expect(calls.of("startup_area")).toHaveLength(2);
});

test("the_startup_area_of_health_shows_this_page_in_place_of_the_generic_checks", async () => {
  const quiet: HealthDto = { ...healthDtoSamples[0], open: [], closed: [] };
  health.dto = quiet;
  answer({ startup_area: () => startupCase("fast") });

  const page = show(Health, { area: "startup" });
  await until(() => page.querySelector(".lede") !== null, "the lede");

  expect(page.textContent).toContain(k("phases"));
  expect(page.textContent).not.toContain(t("ui.health.area.ok_title"));
  expect(page.textContent).not.toContain(t("ui.health.area.none"));
  health.dto = null;
});

beforeEach(() => {
  toast.on = false;
});
