import { afterEach, expect, test } from "vitest";

import { settingsDtoSamples } from "../../dev/fixtures/wire";
import { answer } from "../../dev/test-ipc";
import { show } from "../../dev/test-mount";
import { app } from "../../lib/app.svelte";
import { t } from "../../lib/i18n.svelte";
import { href, parse, router } from "../../lib/router.svelte";
import { updates } from "../../lib/update.svelte";
import Settings from "../Settings.svelte";

afterEach(() => {
  app.settings = null;
  updates.version = "";
  router.commit(parse(""), null);
});

test("the_updates_page_holds_one_row_to_the_release_notes_and_no_notes", async () => {
  const calls = answer({});
  app.settings = { ...settingsDtoSamples[0], editable: true };
  updates.version = "0.1.1";
  router.commit(parse(href.settings("updates")), null);
  const page = show(Settings);
  await new Promise((resolve) => setTimeout(resolve, 20));

  expect(page.querySelector("h1")?.textContent).toBe(t("ui.settings.category.updates"));
  const rows = [...page.querySelectorAll<HTMLAnchorElement>("ul a")];
  expect(rows.map((a) => a.getAttribute("href"))).toEqual([href.news()]);
  expect(rows[0].textContent).toContain(t("ui.settings.updates.news_row"));
  expect(calls.of("release_notes")).toHaveLength(0);
  expect(page.querySelectorAll("article")).toHaveLength(0);

  const version = t("ui.settings.updates.version_hint", { version: "0.1.1" });
  expect(page.textContent?.split(version).length).toBe(2);
});
