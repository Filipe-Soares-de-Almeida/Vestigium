import { expect, test } from "vitest";

import {
  agentLinkSamples,
  liveMessageSamples,
  livePointSamples,
  nowFrameSamples,
  processSampleSamples,
} from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { app } from "../lib/app.svelte";
import { formatClock } from "../lib/format";
import { t } from "../lib/i18n.svelte";
import type { LiveMessage, ProcessSample } from "../lib/types";
import Programs from "./Programs.svelte";

function program(pid: number, name: string, cpu: number): ProcessSample {
  return { ...processSampleSamples[0], pid, name, cpu_pct: cpu };
}

const running = agentLinkSamples.find((link) => link.link === "running" && !link.paused)!;
const programs = [
  program(10, "browser.exe", 40),
  program(11, "browser_helper.exe", 5),
  program(12, "editor.exe", 20),
  program(13, "backup.exe", 1),
];

/** Starts the live stream with the given programs running. */
function stream(entries: ProcessSample[]): void {
  const recent: LiveMessage = { kind: "recent", points: [...livePointSamples] };
  const now = liveMessageSamples[1] as Extract<LiveMessage, { kind: "now" }>;
  const frame = { ...nowFrameSamples[0], programs: { ts_ms: now.frame.programs?.ts_ms ?? 0, entries } };
  answer({
    live_start: (args) => {
      const channel = args.onFrame as { onmessage: (message: LiveMessage) => void };
      setTimeout(() => channel.onmessage(recent), 0);
      setTimeout(() => channel.onmessage({ kind: "now", point: now.point, frame }), 5);
      return 1;
    },
    live_stop: () => null,
    agent_status: () => running,
  });
}

function names(page: HTMLElement): string[] {
  return [...page.querySelectorAll("tbody td.nm")].map((cell) => cell.textContent?.trim() ?? "");
}

function type(page: HTMLElement, text: string): void {
  const field = page.querySelector<HTMLInputElement>("input.field")!;
  field.value = text;
  field.dispatchEvent(new Event("input", { bubbles: true }));
}

test("the_filter_keeps_the_programs_whose_name_holds_the_typed_text", async () => {
  app.link = running;
  stream(programs);
  const page = show(Programs);
  await until(() => names(page).length === 4, "the four programs");
  expect(names(page)).toEqual(["browser.exe", "editor.exe", "browser_helper.exe", "backup.exe"]);

  type(page, "BROWSER");
  await until(() => names(page).length === 2, "the filter to apply");
  expect(names(page)).toEqual(["browser.exe", "browser_helper.exe"]);
  expect(page.querySelector(".note")!.textContent).toBe(t("ui.now.all.count", { shown: 2, total: 2 }));
});

test("the_filter_says_so_when_no_program_matches", async () => {
  app.link = running;
  stream(programs);
  const page = show(Programs);
  await until(() => names(page).length === 4, "the four programs");

  type(page, "nothing like this");
  await until(() => page.querySelector(".empty-cell") !== null, "the empty row");
  expect(page.querySelector(".empty-cell")!.textContent).toBe(t("ui.now.all.none"));
  expect(page.querySelector(".note")!.textContent).toBe(t("ui.now.all.count", { shown: 0, total: 0 }));
});

test("a_silent_monitor_leaves_the_last_programs_faded_under_a_notice", async () => {
  app.link = running;
  stream(programs);
  const page = show(Programs);
  await until(() => names(page).length === 4, "the four programs");

  app.link = { link: "silent", seconds: 3 };
  await until(() => page.querySelector(".silent-note") !== null, "the notice");

  expect(names(page)).toHaveLength(4);
  expect(page.querySelector(".wide")?.classList.contains("veiled")).toBe(true);
  const last = (liveMessageSamples[1] as Extract<LiveMessage, { kind: "now" }>).point!.reading.ts_ms;
  expect(page.querySelector(".silent-note")?.textContent).toContain(t("ui.now.silent.top", { time: formatClock(last) }));
  expect(page.querySelector(".sub")?.textContent).toBe(t("ui.now.lede.silent"));
  expect(page.querySelector(".failure")).toBeNull();
});

test("a_silent_monitor_at_opening_shows_no_rows_and_no_loading", async () => {
  app.link = { link: "silent", seconds: 3 };
  answer({ agent_status: () => ({ link: "silent", seconds: 3 }) });
  const page = show(Programs);

  await until(() => page.querySelector(".silent-note") !== null, "the notice");

  expect(names(page)).toHaveLength(0);
  expect(page.querySelector(".empty-cell")).toBeNull();
  expect(page.querySelector(".silent-note")?.textContent).toContain(t("ui.now.silent.nothing", { time: formatClock(app.answeredAt) }));
});
