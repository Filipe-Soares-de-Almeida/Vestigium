import { flushSync } from "svelte";
import { afterEach, beforeEach, expect, test } from "vitest";

import { startupCase, type Case } from "../dev/fixtures/startup";
import { healthDtoSamples } from "../dev/fixtures/wire";
import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { health } from "../lib/health.svelte";
import { t } from "../lib/i18n.svelte";
import { href, parse, router } from "../lib/router.svelte";
import { startup } from "../lib/startup.svelte";
import type { BootDto, DetailLevel, FindingDto, HealthDto, StartupAreaDto } from "../lib/types";
import Health from "./Health.svelte";
import StartupBoot from "./health/StartupBoot.svelte";
import { startupWhen } from "./health/startup";

const k = (key: string, args?: Record<string, string | number>) => t(`ui.health.startup.${key}`, args);

interface Options {
  level?: DetailLevel;
  /** Changes the data the case answers with. */
  edit?: (dto: StartupAreaDto) => void;
  /** Which startup to open; the latest by default. */
  pick?: (boots: BootDto[]) => BootDto;
}

/** Opens the page of one startup of a case and waits for its title. */
async function open(which: Case, options: Options = {}) {
  const dto = startupCase(which);
  options.edit?.(dto);
  const boot = options.pick ? options.pick(dto.boots) : dto.boots[dto.boots.length - 1];
  answer({ startup_area: () => dto });
  startup.dto = null;
  const page = show(StartupBoot, { boot: boot.kernel_start_ms, level: options.level ?? "Simple" });
  await until(() => page.querySelector("h1") !== null && page.querySelector(".loading") === null, "the title");
  return { page, boot, dto };
}

/** The block whose title is `title`. */
function block(page: HTMLElement, title: string): HTMLElement {
  const found = [...page.querySelectorAll<HTMLElement>("section.block")].find((s) => s.querySelector("h2")?.textContent === title);
  if (!found) throw new Error(`no block ${title}`);
  return found;
}

function click(el: Element | null): void {
  (el as HTMLElement).click();
  flushSync();
}

const kindOf = (el: Element | null) => [...(el?.classList ?? [])].find((c) => ["ok", "info", "attention"].includes(c));
const bars = (page: HTMLElement) => [...page.querySelectorAll(".plot .bar")];
const slowFinding = (): FindingDto => ({ ...healthDtoSamples[0].open[0], kind: "SlowBoot", subject: "Pc" });

beforeEach(() => {
  startup.dto = null;
  startup.error = null;
});

afterEach(() => {
  health.dto = null;
});

test("a_startup_route_carries_the_kernel_start_and_anything_else_has_none", () => {
  expect(parse("#/health/startup/1795000000000")).toEqual({ screen: "health", area: "startup", boot: 1795000000000 });
  expect(parse(href.startupBoot(42))).toEqual({ screen: "health", area: "startup", boot: 42 });
  expect(parse("#/health/startup")).toEqual({ screen: "health", area: "startup", boot: null });
  expect(parse("#/health/startup/abc")).toMatchObject({ screen: "health", area: "startup", boot: null });
  expect(parse("#/health/startup/12x")).toMatchObject({ boot: null });
  expect(parse("#/health/startup/-5")).toMatchObject({ boot: null });
  expect(parse("#/health/disks/55")).toEqual({ screen: "health", area: "disks", boot: null });
  expect(parse("#/health")).toEqual({ screen: "health", area: null, boot: null });
});

test("moving_from_one_startup_to_another_is_a_change_of_screen", () => {
  const key = (hash: string) => {
    router.commit(parse(hash), null);
    return router.key;
  };

  expect(key(href.startupBoot(1))).not.toBe(key(href.startupBoot(2)));
  expect(key(href.startupBoot(1))).not.toBe(key(href.health("startup")));
  router.commit(parse(""), null);
});

test("the_page_of_the_slow_latest_startup_shows_the_profile_in_attention_the_tpm_wait_and_the_temp_count_once", async () => {
  const { page, boot } = await open("slow");

  expect(page.querySelector("h1")?.textContent).toBe(k("boot.title", { when: startupWhen(boot.kernel_start_ms) }));
  const sub = page.querySelector(".head .sub");
  expect(kindOf(sub?.querySelector(".status") ?? null)).toBe("attention");
  expect(sub?.textContent).toContain("3 min 8 s");
  expect(sub?.textContent).toContain(k("boots.total.power", { time: "3 min 8 s" }));
  expect(sub?.textContent).toContain(k("boot.type.full"));

  const profile = block(page, k("boot.profile"));
  expect(profile.textContent).toContain(k("body.tpm_waited", { time: "37 s" }));
  expect(profile.textContent).toContain(k("boot.tpm.after", { time: "115 s" }));
  expect(profile.textContent).toContain("213,879");
  expect(page.querySelector("section.finding")).toBeNull();
});

test("the_phases_are_bars_on_one_axis_and_only_the_profile_bar_takes_the_attention_colour", async () => {
  const { page } = await open("slow");

  const drawn = bars(page);
  expect(drawn).toHaveLength(4);
  expect(drawn.map((bar) => bar.classList.contains("att"))).toEqual([false, false, true, false]);
  const none = page.querySelectorAll(".plot .none");
  expect(none).toHaveLength(1);
  expect(none[0].textContent).toBe(k("value.unmeasured"));
  expect(page.querySelector(".plot .tpm")?.textContent).toContain(t("ui.health.startup.boot.tpm.line"));
  expect(page.querySelector(".cap")?.textContent).toBe(t("ui.health.startup.boot.axis.power"));
  expect(page.querySelector(".plot")?.matches(":focus-visible")).toBe(false);
});

test("the_five_phases_are_listed_under_the_chart_with_their_values_and_marks", async () => {
  const { page } = await open("slow");
  const phases = block(page, k("boot.phases"));

  const rows = [...phases.querySelectorAll<HTMLElement>("ul.rows li")];
  expect(rows).toHaveLength(5);
  expect(rows[2].textContent).toContain("2 min 32 s");
  expect(kindOf(rows[2].querySelector(".status"))).toBe("attention");
  expect(rows[4].textContent).toContain(k("value.unmeasured"));
  expect(phases.querySelector("button[aria-expanded]")).toBeNull();
  expect(phases.querySelector(".mini")).toBeNull();
  expect(phases.querySelector(".zoombar")).not.toBeNull();
});

test("a_wait_on_the_sign_in_screen_is_a_lighter_hatched_bar_and_the_readout_names_it", async () => {
  const { page } = await open("wait");

  expect(bars(page)[1].classList.contains("wait")).toBe(true);
  expect(bars(page).some((bar) => bar.classList.contains("att"))).toBe(true);
  const plot = page.querySelector<HTMLElement>(".plot")!;
  plot.focus();
  flushSync();
  plot.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
  flushSync();

  const tip = page.querySelector(".tip");
  expect(tip?.textContent).toContain(k("phase.signin"));
  expect(tip?.textContent).toContain(k("word.wait"));
  expect(tip?.textContent).toContain("12 min");
  expect(page.querySelector(".lab.on")?.textContent).toBe(k("phase.signin"));
});

test("the_keyboard_reads_the_phases_row_by_row_with_the_tpm_wait_on_the_profile_row", async () => {
  const { page } = await open("slow");
  const plot = page.querySelector<HTMLElement>(".plot")!;
  const press = (key: string) => {
    plot.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flushSync();
  };

  plot.focus();
  flushSync();
  expect(page.querySelector(".tip")?.textContent).toContain(k("phase.profile"));
  expect(page.querySelector(".tip")?.textContent).toContain(t("ui.health.startup.boot.tip.tpm"));
  expect(page.querySelector(".tip")?.textContent).toContain(k("range.profile", { limit: 30 }));
  press("End");
  expect(page.querySelector(".tip")?.textContent).toContain(k("body.unmeasured"));
  expect(page.querySelector(".plot .bar.on")).toBeNull();
  press("Home");
  expect(page.querySelector(".tip")?.textContent).toContain(k("phase.firmware"));
  expect(page.querySelector(".plot .bar.on")).not.toBeNull();
  press("Escape");
  expect(page.querySelector(".tip")).toBeNull();
});

test("the_chart_zooms_with_the_plus_key_and_the_zoom_bar_resets_it", async () => {
  const { page } = await open("fast");
  const plot = page.querySelector<HTMLElement>(".plot")!;
  const reset = [...page.querySelectorAll<HTMLButtonElement>(".zoombar button")].find((b) => b.textContent === t("ui.zoom.reset"))!;
  const ticks = () => page.querySelectorAll(".axis span").length;
  const before = [...page.querySelectorAll(".axis span")].map((s) => s.textContent);

  expect(reset.disabled).toBe(true);
  plot.focus();
  plot.dispatchEvent(new KeyboardEvent("keydown", { key: "+", bubbles: true }));
  flushSync();
  expect(reset.disabled).toBe(false);
  expect([...page.querySelectorAll(".axis span")].map((s) => s.textContent)).not.toEqual(before);
  expect(ticks()).toBeGreaterThan(1);

  click(reset);
  expect(reset.disabled).toBe(true);
  expect([...page.querySelectorAll(".axis span")].map((s) => s.textContent)).toEqual(before);
});

test("an_older_startup_has_no_temp_count_and_links_to_the_startups_around_it", async () => {
  const { page, dto } = await open("slow", { pick: (boots) => boots[boots.length - 3] });
  const at = dto.boots.length - 3;

  const profile = block(page, k("boot.profile"));
  expect(profile.textContent).not.toContain(k("temp.title"));
  const links = [...page.querySelectorAll<HTMLAnchorElement>("nav.pn a")];
  expect(links.map((a) => a.getAttribute("href"))).toEqual([
    href.startupBoot(dto.boots[at - 1].kernel_start_ms),
    href.startupBoot(dto.boots[at + 1].kernel_start_ms),
  ]);
  expect(links[0].textContent).toBe(k("boot.prev", { when: startupWhen(dto.boots[at - 1].kernel_start_ms) }));
  expect(links[1].textContent).toBe(k("boot.next", { when: startupWhen(dto.boots[at + 1].kernel_start_ms) }));
});

test("the_first_and_the_latest_startup_each_miss_one_neighbour", async () => {
  const first = await open("slow", { pick: (boots) => boots[0] });
  expect([...first.page.querySelectorAll("nav.pn a")].map((a) => a.textContent)).toHaveLength(1);
  expect(first.page.querySelector("nav.pn a")?.textContent).toContain(k("boot.next", { when: "" }).slice(0, 8));

  const last = await open("slow");
  const lastLinks = [...last.page.querySelectorAll("nav.pn a")];
  expect(lastLinks).toHaveLength(1);
  expect(lastLinks[0].textContent).toContain(k("boot.prev", { when: "" }).slice(0, 8));
});

test("a_startup_without_a_firmware_time_counts_from_windows_and_says_so_with_a_not_measured_row", async () => {
  const { page } = await open("fast", { pick: (boots) => boots.find((b) => b.firmware_ms === null)! });

  expect(page.querySelector(".cap")?.textContent).toBe(t("ui.health.startup.boot.axis.windows"));
  const none = page.querySelectorAll(".plot .none");
  expect(none[0].textContent).toBe(k("value.unmeasured"));
  expect(bars(page)[0].getAttribute("style")).toContain("left: 0%");
  expect(page.querySelector(".head .sub")?.textContent).toContain(k("boots.total.none"));
});

test("a_startup_that_is_not_in_the_history_says_so_and_keeps_the_back_link", async () => {
  answer({ startup_area: () => startupCase("fast") });
  const page = show(StartupBoot, { boot: 12345, level: "Simple" });
  await until(() => page.querySelector("h1") !== null && page.querySelector(".loading") === null, "the title");

  expect(page.querySelector("h1")?.textContent).toBe(t("ui.health.startup.boot.missing.title"));
  expect(page.textContent).toContain(t("ui.health.startup.boot.missing.body"));
  expect(page.querySelector("a.back")?.getAttribute("href")).toBe(href.health("startup"));
  expect(page.querySelector(".plot")).toBeNull();
});

test("the_back_link_names_the_area_and_leads_to_it", async () => {
  const { page } = await open("fast");

  const back = page.querySelector("a.back");
  expect(back?.getAttribute("href")).toBe(href.health("startup"));
  expect(back?.textContent).toContain(t("ui.health.area.startup"));
});

test("a_startup_with_no_apps_or_failures_says_so_and_one_with_them_lists_them", async () => {
  const none = await open("fast", { pick: (boots) => boots.find((b) => b.firmware_ms === null)! });
  expect(block(none.page, k("apps")).textContent).toContain(k("body.unmeasured"));
  expect(block(none.page, k("fail.title")).textContent).toContain(t("ui.health.startup.boot.fail.none"));

  const some = await open("fast");
  const apps = block(some.page, k("apps"));
  expect(apps.textContent).toContain("vestigium.exe");
  expect(apps.textContent).toContain(k("run.opened", { at: "46.9 s" }));

  const slow = await open("slow");
  const dam = [...block(slow.page, k("fail.title")).querySelectorAll("li")].find((li) => li.textContent?.includes("\"dam\""))!;
  click(dam.querySelector("button"));
  expect(dam.textContent).toContain(k("fail.dam").slice(0, 30));
  expect(dam.textContent).toContain(t("ui.health.startup.boot.fail.sub"));
});

test("the_technical_level_names_the_sources_and_a_full_windows_log_and_the_simple_level_does_not", async () => {
  const edit = (dto: StartupAreaDto) => {
    dto.boots[dto.boots.length - 1].logger_full = true;
  };
  const technical = await open("slow", { level: "Technical", edit });
  expect(technical.page.textContent).toContain(k("boot.sources", { names: "Kernel-Boot 30, Winlogon 7001, User Profile Service, Shell-Core 62171, Shell-Core 9707" }));
  expect(technical.page.textContent).toContain(t("ui.health.startup.boot.logger_full"));

  const simple = await open("slow", { edit });
  expect(simple.page.textContent).not.toContain("Kernel-Boot 30");
  expect(simple.page.textContent).not.toContain(t("ui.health.startup.boot.logger_full"));
});

test("the_page_shows_what_was_read_before_while_it_reads_again_and_keeps_it_when_the_read_fails", async () => {
  const dto = startupCase("slow");
  startup.dto = dto;
  answer({ startup_area: () => Promise.reject(refusal("store")) });

  const page = show(StartupBoot, { boot: dto.boots[dto.boots.length - 1].kernel_start_ms, level: "Simple" });

  expect(page.querySelector(".plot")).not.toBeNull();
  expect(page.querySelector(".loading")).toBeNull();
  await new Promise((resolve) => setTimeout(resolve, 20));
  expect(page.querySelector(".failure")).toBeNull();
  expect(page.querySelector(".plot")).not.toBeNull();
});

test("a_failed_first_read_shows_the_error_and_try_again_loads_the_page", async () => {
  let failing = true;
  const dto = startupCase("fast");
  answer({ startup_area: () => (failing ? Promise.reject(refusal("store")) : dto) });
  const page = show(StartupBoot, { boot: dto.boots[dto.boots.length - 1].kernel_start_ms, level: "Simple" });
  await until(() => page.querySelector(".failure") !== null, "the error");

  failing = false;
  click(page.querySelector(".failure .btn"));
  await until(() => page.querySelector(".plot") !== null, "the chart");
});

test("health_opens_the_page_of_a_startup_in_place_of_the_area_without_repeating_the_finding", async () => {
  const quiet: HealthDto = { ...healthDtoSamples[0], open: [slowFinding()], closed: [] };
  health.dto = quiet;
  const dto = startupCase("slow");
  answer({ startup_area: () => dto });

  const page = show(Health, { area: "startup", boot: dto.boots[dto.boots.length - 1].kernel_start_ms });
  await until(() => page.querySelector(".plot") !== null, "the chart");

  expect(page.querySelectorAll("h1")).toHaveLength(1);
  expect(page.querySelector("h1")?.textContent).toContain(startupWhen(dto.boots[dto.boots.length - 1].kernel_start_ms));
  expect(page.textContent).not.toContain(t("ui.health.area.ok_title"));
  expect(page.querySelector(".lede")).toBeNull();
  expect(page.querySelector("section.finding")).toBeNull();
});
