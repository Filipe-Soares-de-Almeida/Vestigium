import { expect, test } from "vitest";

import { capture } from "../dev/fixtures/capture";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { t } from "../lib/i18n.svelte";
import type { CaptureDto } from "../lib/types";
import Capture from "./Capture.svelte";

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
