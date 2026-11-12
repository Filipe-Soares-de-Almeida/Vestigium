import { afterEach, expect, test } from "vitest";

import { answer } from "../../dev/test-ipc";
import { show, until } from "../../dev/test-mount";
import { buildGroups, groupOf } from "../../lib/groups";
import { t } from "../../lib/i18n.svelte";
import { href, parse } from "../../lib/router.svelte";
import { type Release, updates } from "../../lib/update.svelte";
import News from "./News.svelte";
import NewsVersion from "./NewsVersion.svelte";

/** Entries named by letters, so a count on screen can only come from the page itself. */
const items = (kind: string, n: number) =>
  Array.from({ length: n }, (_, i) => `${kind} entry ${String.fromCharCode(97 + (i % 26))}${i < 26 ? "" : "z"}.`);

function release(version: string, counts: Record<string, number>, summary: string | null = null): Release {
  return {
    version,
    date: "2026-11-11",
    summary,
    groups: Object.entries(counts).map(([kind, n]) => ({ kind, items: items(kind, n) })),
  };
}

const NOTES = [release("0.1.1", { Added: 6, Changed: 12, Fixed: 44, Security: 8 }), release("0.1.0", { Added: 9 }, "The first release.")];

afterEach(() => {
  updates.version = "";
  updates.found = null;
});

function serve(notes: Release[] = NOTES) {
  updates.version = "0.1.1";
  return answer({ release_notes: () => notes });
}

const cards = (page: HTMLElement) => [...page.querySelectorAll<HTMLElement>("article")];
const button = (page: HTMLElement, text: string) =>
  [...page.querySelectorAll("button")].find((b) => b.textContent?.trim() === text);

test("the_news_routes_sit_under_updates_and_keep_the_pane_on_it", () => {
  expect(href.news()).toBe("#/settings/updates/news");
  expect(href.news("0.1.1")).toBe("#/settings/updates/news/0.1.1");
  expect(href.news("0.1.1", "Fixed-3")).toBe("#/settings/updates/news/0.1.1?entry=Fixed-3");

  expect(parse(href.news())).toEqual({ screen: "settings", category: "updates", news: { version: null, entry: null } });
  expect(parse(href.news("0.1.1", "Fixed-3"))).toEqual({
    screen: "settings",
    category: "updates",
    news: { version: "0.1.1", entry: "Fixed-3" },
  });
  expect(parse("#/settings/updates")).toEqual({ screen: "settings", category: "updates", news: null });

  const groups = buildGroups();
  for (const route of [parse(href.news()), parse(href.news("0.1.1"))]) {
    const group = groupOf(groups, route, null);
    expect(group?.key).toBe("settings");
    expect(group?.items.filter((i) => i.match(route)).map((i) => i.href)).toEqual([href.settings("updates")]);
  }
});

test("the_wall_shows_a_card_per_version_newest_first_with_the_installed_tag", async () => {
  serve();
  const page = show(News);
  await until(() => cards(page).length === 2, "the cards");

  expect(page.querySelector("h1")?.textContent).toBe(t("ui.news.title"));
  expect(page.querySelector(".back")?.getAttribute("href")).toBe(href.settings("updates"));
  const [newest, first] = cards(page);
  expect(newest.querySelector("h2")?.textContent).toBe(t("ui.update.version", { version: "0.1.1" }));
  expect(newest.textContent).toContain(t("ui.news.tag.installed"));
  expect(first.textContent).not.toContain(t("ui.news.tag.installed"));
  expect(first.textContent).toContain("The first release.");
});

test("each_card_lists_five_highlights_that_open_the_version_on_their_entry", async () => {
  serve();
  const page = show(News);
  await until(() => cards(page).length === 2, "the cards");

  const links = [...cards(page)[0].querySelectorAll<HTMLAnchorElement>("li a")];
  expect(links.map((a) => a.getAttribute("href"))).toEqual([
    ...[0, 1, 2, 3].map((i) => href.news("0.1.1", `Added-${i}`)),
    href.news("0.1.1", "Security-0"),
  ]);
  expect(cards(page)[1].querySelectorAll("li a")).toHaveLength(5);
});

test("the_chips_are_the_only_counts_on_a_card", async () => {
  serve();
  const page = show(News);
  await until(() => cards(page).length === 2, "the cards");

  const card = cards(page)[0];
  const chips = [...card.querySelectorAll(".cnt")].map((c) => c.textContent?.replace(/\s+/g, " ").trim());
  expect(chips).toEqual(["New 6", "Changed 12", "Fixed 44", "Security 8"]);
  const all = [...card.querySelectorAll<HTMLAnchorElement>("a")].find((a) => a.textContent === t("ui.news.see_all"));
  expect(all?.getAttribute("href")).toBe(href.news("0.1.1"));

  const rest = card.cloneNode(true) as HTMLElement;
  rest.querySelector(".cnts")?.remove();
  expect(rest.textContent).not.toMatch(/\b(6|12|44|8)\b/);
});

test("a_version_found_by_a_check_tops_the_wall_as_available", async () => {
  serve();
  updates.found = { status: "available", version: "0.1.2", date_ms: null, summary: "Shorter notes.", groups: [{ kind: "Added", items: ["A page."] }] };
  const page = show(News);
  await until(() => cards(page).length === 3, "the cards");

  expect(cards(page)[0].textContent).toContain(t("ui.news.tag.available"));
  expect(cards(page)[0].textContent).toContain("Shorter notes.");
  expect(cards(page)[1].textContent).toContain(t("ui.news.tag.installed"));
});

test("a_long_group_shows_10_entries_and_the_rest_on_request", async () => {
  serve();
  const page = show(NewsVersion, { version: "0.1.1", entry: null });
  await until(() => page.querySelector("section") !== null, "the groups");

  expect(page.querySelector(".back")?.getAttribute("href")).toBe(href.news());
  expect(page.querySelector(".sub")?.textContent).toContain(t("ui.news.tag.installed"));
  const fixed = page.querySelector<HTMLElement>("section[aria-labelledby='group-Fixed']")!;
  expect(fixed.querySelector("h2")?.textContent).toContain("44 changes");
  expect(fixed.querySelectorAll("li")).toHaveLength(10);
  const changed = page.querySelector<HTMLElement>("section[aria-labelledby='group-Changed']")!;
  expect(changed.querySelectorAll("li")).toHaveLength(12);
  expect(changed.querySelector("button")).toBeNull();

  const more = button(page, t("ui.news.show_rest", { count: 34 }))!;
  expect(more).toBeDefined();
  more.click();
  await until(() => fixed.querySelectorAll("li").length === 44, "the whole group");
  expect(document.activeElement?.id).toBe("entry-Fixed-10");
  expect(button(page, t("ui.news.show_rest", { count: 34 }))).toBeUndefined();
});

test("opening_on_an_entry_past_the_cut_shows_its_whole_group_and_marks_it", async () => {
  serve();
  const page = show(NewsVersion, { version: "0.1.1", entry: "Fixed-30" });
  await until(() => page.querySelector("section") !== null, "the groups");

  const fixed = page.querySelector<HTMLElement>("section[aria-labelledby='group-Fixed']")!;
  expect(fixed.querySelectorAll("li")).toHaveLength(44);
  expect(page.querySelectorAll("li.target")).toHaveLength(1);
  expect(page.querySelector("li.target")?.id).toBe("entry-Fixed-30");
});

test("the_offered_version_page_offers_the_update_and_an_installed_one_does_not", async () => {
  serve();
  updates.found = { status: "available", version: "0.1.2", date_ms: null, summary: null, groups: [{ kind: "Added", items: ["A page."] }] };
  const offered = show(NewsVersion, { version: "0.1.2", entry: null });
  await until(() => offered.querySelector("section") !== null, "the offered version");
  expect(button(offered, t("ui.update.available.install"))).toBeDefined();
  expect(button(offered, t("ui.news.copy"))).toBeDefined();

  const installed = show(NewsVersion, { version: "0.1.1", entry: null });
  await until(() => installed.querySelector("section") !== null, "the installed version");
  expect(button(installed, t("ui.update.available.install"))).toBeUndefined();
});

test("an_unknown_version_says_it_has_no_notes", async () => {
  serve();
  const page = show(NewsVersion, { version: "9.9.9", entry: null });
  await until(() => page.querySelector(".empty") !== null, "the message");
  expect(page.querySelector(".empty")?.textContent).toBe(t("ui.news.missing", { version: "9.9.9" }));
});
