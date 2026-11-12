import { afterEach, beforeEach, expect, test } from "vitest";

import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { type AboutVersions, COMPONENTS, filterRows, allRows } from "../lib/about";
import { aboutDialog } from "../lib/aboutDialog.svelte";
import { app } from "../lib/app.svelte";
import { t } from "../lib/i18n.svelte";
import { updates } from "../lib/update.svelte";
import Home from "../screens/Home.svelte";
import AboutDialog from "./AboutDialog.svelte";
import NavPane from "./NavPane.svelte";

const SAME: AboutVersions = {
  version: "0.1.1",
  released: "2026-11-11",
  monitor: { run: "running", version: "0.1.1", protocol: null },
  helper: "0.1.1",
  schema: 13,
  protocol: 22,
  windows: { edition: "Windows 11 Pro", family: "Windows 11", release: "24H2", build: "26100.2605", version: "10.0.26100.2605" },
  webview: "131.0.2903.112",
};

beforeEach(() => {
  updates.version = "0.1.1";
  app.settings = {
    settings: { language: "System", detail_level: "Detailed" },
    autostart: null,
    editable: false,
  } as never;
});

afterEach(() => {
  aboutDialog.open = false;
  aboutDialog.facts = null;
  aboutDialog.failure = null;
  updates.version = "";
  app.settings = null;
});

/** Opens the dialog with `facts` and waits until it shows them. */
async function opened(facts: AboutVersions = SAME): Promise<HTMLDialogElement> {
  answer({ about_versions: () => facts });
  const page = show(AboutDialog);
  aboutDialog.show();
  await until(() => page.querySelector("dialog")?.open === true && page.querySelector("dl") !== null, "the dialog");
  return page.querySelector("dialog")!;
}

/** The value beside the label `name` in the dialog's lists. */
function value(dialog: HTMLElement, name: string): HTMLElement | null {
  const term = [...dialog.querySelectorAll("dt")].find((dt) => dt.textContent?.trim() === name);
  return (term?.nextElementSibling as HTMLElement | null) ?? null;
}

const text = (element: Element | null) => element?.textContent?.replace(/\s+/g, " ").trim() ?? "";

test("the_name_atop_the_pane_the_version_in_its_foot_and_the_home_version_each_open_the_dialog", () => {
  answer({ about_versions: () => SAME });
  const pane = show(NavPane);
  const name = pane.querySelector<HTMLButtonElement>("button.name")!;
  expect(name.getAttribute("aria-label")).toBe(t("ui.about.title"));
  name.click();
  expect(aboutDialog.open).toBe(true);

  aboutDialog.open = false;
  const foot = pane.querySelector<HTMLButtonElement>(".foot button.ver")!;
  expect(text(foot)).toBe(t("ui.settings.updates.version_hint", { version: "0.1.1" }));
  foot.click();
  expect(aboutDialog.open).toBe(true);

  aboutDialog.open = false;
  const home = show(Home);
  const line = home.querySelector<HTMLButtonElement>(".state button.ver")!;
  expect(text(line)).toBe(t("ui.settings.updates.version_hint", { version: "0.1.1" }));
  line.click();
  expect(aboutDialog.open).toBe(true);
});

test("the_header_says_the_version_and_its_release_date_once", async () => {
  const dialog = await opened();

  expect(text(dialog.querySelector("#about-line"))).toBe(t("ui.about.released", { version: "0.1.1", date: "November 11, 2026" }));
  expect(text(dialog.querySelector(".head"))).toContain(t("ui.about.tagline"));
  expect(text(value(dialog, t("ui.about.window")))).toBe("Vestigium.Window.exe");
});

test("escape_and_close_both_close_the_dialog", async () => {
  const dialog = await opened();

  dialog.dispatchEvent(new Event("cancel", { cancelable: true }));
  expect(aboutDialog.open).toBe(false);

  aboutDialog.show();
  await until(() => dialog.open, "the dialog again");
  [...dialog.querySelectorAll<HTMLButtonElement>(".foot button")].find((b) => text(b) === t("ui.about.close"))!.click();
  expect(aboutDialog.open).toBe(false);
});

test("arrow_keys_move_between_the_tabs_and_select_the_one_they_reach", async () => {
  const dialog = await opened();
  const [versions, credits] = [...dialog.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  await until(() => document.activeElement === versions, "focus on the first tab");
  expect(versions.getAttribute("aria-selected")).toBe("true");
  expect(credits.tabIndex).toBe(-1);

  versions.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
  await until(() => credits.getAttribute("aria-selected") === "true", "the credits tab");
  expect(document.activeElement).toBe(credits);
  expect(credits.tabIndex).toBe(0);
  expect(dialog.querySelector('[role="tabpanel"]')?.getAttribute("aria-labelledby")).toBe("about-tab-credits");
  expect(text(dialog)).toContain(t("ui.about.components"));

  credits.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
  await until(() => versions.getAttribute("aria-selected") === "true", "back to versions");
  credits.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
  await until(() => document.activeElement === credits, "the last tab");
  credits.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true }));
  await until(() => document.activeElement === versions, "the first tab");
});

test("a_program_of_this_version_shows_no_version_of_its_own", async () => {
  const dialog = await opened();

  const monitor = value(dialog, t("ui.about.monitor"));
  expect(text(monitor)).toBe(`${t("ui.about.monitor.running")} Vestigium.exe`);
  expect(monitor?.querySelector(".status")).toBeNull();
  expect(text(value(dialog, t("ui.about.helper")))).toBe(`${t("ui.about.helper.on_demand")} Vestigium.Helper.exe`);
  expect(dialog.querySelector(".hint")).toBeNull();
});

test("a_program_of_another_version_shows_it_and_the_monitor_gets_a_mark", async () => {
  const dialog = await opened({ ...SAME, monitor: { run: "running", version: "0.1.0", protocol: null }, helper: "0.1.0" });

  const monitor = value(dialog, t("ui.about.monitor"));
  expect(text(monitor)).toContain("0.1.0");
  expect(text(monitor)).toContain(t("ui.about.monitor.running"));
  expect(monitor?.querySelector(".status.attention")).not.toBeNull();
  const helper = value(dialog, t("ui.about.helper"));
  expect(text(helper)).toContain("0.1.0");
  expect(helper?.querySelector(".status")).toBeNull();
  expect(text(dialog.querySelector(".hint"))).toContain(t("ui.about.monitor.earlier"));
});

test("a_stale_monitor_names_its_protocol_and_says_how_to_restart_it", async () => {
  const dialog = await opened({ ...SAME, monitor: { run: "other", version: null, protocol: 13 } });

  const monitor = value(dialog, t("ui.about.monitor"));
  expect(text(monitor)).toContain(t("ui.about.monitor.other"));
  expect(text(monitor)).toContain(t("ui.about.monitor.speaks", { protocol: 13, expected: 22 }));
  expect(monitor?.querySelector(".status.attention")).not.toBeNull();
  expect(text(value(dialog, t("ui.about.protocol")))).toBe("22");
  expect(text(dialog.querySelector(".hint"))).toContain(t("ui.about.monitor.earlier"));
});

test("a_stopped_monitor_says_so_and_has_no_hint", async () => {
  const dialog = await opened({ ...SAME, monitor: { run: "stopped", version: "0.1.1", protocol: null } });

  expect(text(value(dialog, t("ui.about.monitor")))).toBe(`${t("ui.about.monitor.stopped")} Vestigium.exe`);
  expect(dialog.querySelector(".hint")).toBeNull();
});

test("this_pc_lists_windows_webview_the_language_and_the_detail_level", async () => {
  const dialog = await opened();

  expect(text(value(dialog, t("ui.about.windows")))).toBe("Windows 11 Pro, 24H2");
  expect(text(value(dialog, t("ui.about.build")))).toBe("26100.2605");
  expect(text(value(dialog, t("ui.about.webview")))).toBe("131.0.2903.112");
  expect(text(value(dialog, t("ui.about.language")))).toBe(`${t("ui.settings.language.En")} ${t("ui.about.as_windows")}`);
  expect(text(value(dialog, t("ui.common.detail.label")))).toBe(t("ui.common.detail.Detailed"));
});

test("what_was_not_read_is_left_out_rather_than_guessed", async () => {
  const dialog = await opened({ ...SAME, released: null, helper: null, windows: null, webview: null });

  expect(text(dialog.querySelector("#about-line"))).toBe(t("ui.update.version", { version: "0.1.1" }));
  expect(value(dialog, t("ui.about.windows"))).toBeNull();
  expect(value(dialog, t("ui.about.build"))).toBeNull();
  expect(value(dialog, t("ui.about.webview"))).toBeNull();
  expect(text(value(dialog, t("ui.about.helper")))).toBe(`${t("ui.about.helper.missing")} Vestigium.Helper.exe`);
});

test("the_component_count_shows_only_while_a_filter_narrows_the_list", async () => {
  const dialog = await opened();
  dialog.querySelector<HTMLButtonElement>("#about-tab-credits")!.click();
  await until(() => dialog.querySelector(".seg") !== null, "the credits");

  const [main, all] = [...dialog.querySelectorAll<HTMLButtonElement>(".seg button")];
  const total = COMPONENTS.length;
  expect(text(main)).toBe(t("ui.about.components.main", { count: COMPONENTS.filter((c) => c.main).length }));
  expect(text(all)).toBe(t("ui.about.components.all", { count: total }));
  expect(dialog.querySelectorAll("tbody tr")).toHaveLength(COMPONENTS.filter((c) => c.main).length);
  expect(text(dialog.querySelector("tbody td"))).toBe(`Tauri${t("ui.about.component.tauri.caption")}`);
  expect(text(dialog.querySelector(".count"))).toBe("");

  all.click();
  await until(() => dialog.querySelectorAll("tbody tr").length === total, "every component");
  expect(text(dialog.querySelector(".count"))).toBe("");
  const field = dialog.querySelector<HTMLInputElement>("#about-filter")!;
  await until(() => document.activeElement === field, "focus on the filter");

  field.value = "tauri";
  field.dispatchEvent(new Event("input", { bubbles: true }));
  const shown = filterRows(allRows(), "tauri").length;
  await until(() => dialog.querySelectorAll("tbody tr").length === shown, "the filtered list");
  expect(text(dialog.querySelector(".count"))).toBe(t("ui.about.components.shown", { shown, total }));

  field.value = "no such thing";
  field.dispatchEvent(new Event("input", { bubbles: true }));
  await until(() => text(dialog.querySelector("tbody")) === t("ui.about.components.none"), "the empty list");
});
