import { afterEach, expect, test } from "vitest";

import { show, until } from "../dev/test-mount";
import { t } from "../lib/i18n.svelte";
import { href } from "../lib/router.svelte";
import { type Release, updates } from "../lib/update.svelte";
import UpdateDialog from "./UpdateDialog.svelte";

const GROUPS = [
  { kind: "Added", items: ["A page.", "A dialog.", "A button."] },
  { kind: "Changed", items: ["Shorter notes."] },
  { kind: "Fixed", items: ["A fix.", "Another fix."] },
  { kind: "Security", items: ["Refuse large notes."] },
];

const release = (version: string, summary: string | null = null): Release => ({ version, date: "2026-11-25", summary, groups: GROUPS });

afterEach(() => {
  updates.dialog = null;
  updates.found = null;
  updates.news = [];
  updates.version = "";
  updates.download = null;
});

function open(page: HTMLElement): HTMLDialogElement | null {
  const dialog = page.querySelector("dialog");
  return dialog?.open ? dialog : null;
}

const highlights = (dialog: HTMLElement) => [...dialog.querySelectorAll("article li a")].map((a) => a.textContent?.trim());

test("an_offered_version_shows_its_card_with_three_highlights_and_no_header", async () => {
  updates.version = "0.1.1";
  updates.found = { status: "available", version: "0.1.2", date_ms: null, summary: "Notes in their own place.", groups: GROUPS };
  updates.dialog = "available";
  const page = show(UpdateDialog);
  await until(() => open(page) !== null, "the dialog");

  const dialog = open(page)!;
  expect(dialog.querySelector("h2")?.textContent).toBe(t("ui.update.available.title", { version: "0.1.2" }));
  expect(dialog.textContent).toContain(t("ui.update.available.body", { current: "0.1.1" }));
  expect(dialog.querySelector("article h2")).toBeNull();
  expect(dialog.textContent).toContain("Notes in their own place.");
  expect(highlights(dialog)).toEqual(["A page.", "A dialog.", "Refuse large notes."]);
  expect([...dialog.querySelectorAll(".cnt")].map((c) => c.textContent?.replace(/\s+/g, " ").trim())).toEqual([
    "New 3",
    "Changed 1",
    "Fixed 2",
    "Security 1",
  ]);
  const buttons = [...dialog.querySelectorAll("button")].map((b) => b.textContent?.trim());
  expect(buttons).toEqual([t("ui.update.available.later"), t("ui.update.available.install")]);
});

test("the_card_link_opens_the_version_page_and_closes_the_dialog", async () => {
  updates.version = "0.1.1";
  updates.found = { status: "available", version: "0.1.2", date_ms: null, summary: null, groups: GROUPS };
  updates.dialog = "available";
  const page = show(UpdateDialog);
  await until(() => open(page) !== null, "the dialog");

  const all = [...open(page)!.querySelectorAll<HTMLAnchorElement>("a")].find((a) => a.textContent === t("ui.news.see_all"))!;
  expect(all.getAttribute("href")).toBe(href.news("0.1.2"));
  all.click();
  await until(() => open(page) === null, "the dialog to close");
  expect(updates.dialog).toBeNull();
});

test("the_card_links_are_out_of_reach_while_the_update_downloads", async () => {
  updates.version = "0.1.1";
  updates.found = { status: "available", version: "0.1.2", date_ms: null, summary: null, groups: GROUPS };
  updates.dialog = "available";
  const page = show(UpdateDialog);
  await until(() => open(page) !== null, "the dialog");
  const inert = () => {
    const box = open(page)!.querySelector("article")!.parentElement as HTMLElement & { inert?: boolean };
    return box.inert === true || box.hasAttribute("inert");
  };
  expect(inert()).toBe(false);

  updates.download = { done: 10, total: 100 };
  await until(inert, "the card to be inert");
  expect(open(page)!.querySelector("[role=progressbar]")).not.toBeNull();
});

test("after_an_update_the_dialog_shows_the_newest_version_card", async () => {
  updates.version = "0.1.2";
  updates.news = [release("0.1.2", "Notes in their own place.")];
  updates.dialog = "news";
  const page = show(UpdateDialog);
  await until(() => open(page) !== null, "the dialog");

  const dialog = open(page)!;
  expect(dialog.querySelector("h2")?.textContent).toBe(t("ui.update.news.title", { version: "0.1.2" }));
  expect(dialog.querySelectorAll("article")).toHaveLength(1);
  expect(highlights(dialog)).toHaveLength(3);
  expect(dialog.querySelector(`a[href="${href.news()}"]`)).toBeNull();
});

test("several_new_versions_add_a_link_to_the_release_notes_page", async () => {
  updates.version = "0.1.3";
  updates.news = [release("0.1.3"), release("0.1.2")];
  updates.dialog = "news";
  const page = show(UpdateDialog);
  await until(() => open(page) !== null, "the dialog");

  const dialog = open(page)!;
  expect(dialog.querySelectorAll("article")).toHaveLength(1);
  expect(dialog.querySelector("article a")?.getAttribute("href")).toBe(href.news("0.1.3", "Added-0"));
  const every = dialog.querySelector<HTMLAnchorElement>(`a[href="${href.news()}"]`)!;
  expect(every.textContent).toBe(t("ui.settings.updates.news_row"));
  every.click();
  await until(() => open(page) === null, "the dialog to close");
});
