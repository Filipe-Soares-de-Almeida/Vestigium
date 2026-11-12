import { afterEach, expect, test, vi } from "vitest";

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
import type { AgentLink, LiveMessage } from "../lib/types";
import Now from "./Now.svelte";

// jsdom has no canvas for the chart library.
vi.mock("./now/chart", async (original) => ({
  ...(await original<typeof import("./now/chart")>()),
  laneChart: () => ({ set: () => {}, destroy: () => {}, readAt: () => {} }),
}));

const running = agentLinkSamples.find((link) => link.link === "running" && !link.paused)!;
const silent: AgentLink = { link: "silent", seconds: 3 };

afterEach(() => {
  app.link = null;
});

/** A live stream that sends the last ten minutes and one second, then whatever `after` says. */
function stream(after: LiveMessage | null, status: () => AgentLink = () => running): { starts: () => number } {
  const recent: LiveMessage = { kind: "recent", points: [...livePointSamples] };
  const now = liveMessageSamples[1] as Extract<LiveMessage, { kind: "now" }>;
  const frame = {
    ...nowFrameSamples[0],
    programs: { ts_ms: 0, entries: [{ ...processSampleSamples[0], name: "browser.exe" }] },
    hung: [],
  };
  let starts = 0;
  answer({
    live_start: (args) => {
      starts++;
      const channel = args.onFrame as { onmessage: (message: LiveMessage) => void };
      setTimeout(() => channel.onmessage(recent), 0);
      setTimeout(() => channel.onmessage({ kind: "now", point: now.point, frame }), 5);
      if (after) setTimeout(() => channel.onmessage(after), 10);
      return 1;
    },
    live_stop: () => null,
    agent_status: status,
    settings: () => null,
  });
  return { starts: () => starts };
}

const notes = (page: HTMLElement): string[] =>
  [...page.querySelectorAll(".silent-note")].map((note) => note.textContent?.trim() ?? "");

test("a_watcher_that_stops_answering_keeps_the_last_values_faded_under_their_notices", async () => {
  app.link = running;
  let link: AgentLink = running;
  stream({ kind: "silent" }, () => link);
  const page = show(Now);
  await until(() => page.querySelector(".tbl") !== null, "the programs");
  expect(notes(page)).toEqual([]);

  link = silent;
  await app.refresh();
  await until(() => notes(page).length === 3, "a notice on each live block");

  expect(page.querySelectorAll(".block.dim")).toHaveLength(3);
  expect(page.querySelector(".tbl")?.textContent).toContain("browser.exe");
  expect(page.querySelector(".sub")?.textContent).toBe(t("ui.now.lede.silent"));
  expect(page.querySelector(".failure")).toBeNull();
  expect(page.querySelector(".banner")).toBeNull();
});

test("programs_not_responding_says_it_cannot_tell_instead_of_none", async () => {
  app.link = running;
  stream({ kind: "silent" }, () => silent);
  const page = show(Now);

  await until(() => notes(page).length === 3, "the notices");

  const text = page.textContent ?? "";
  expect(notes(page)).toContain(t("ui.now.silent.hung"));
  expect(text).not.toContain(t("ui.now.hung.none"));
  expect(text).toContain(t("ui.now.tenmin.silent"));
});

test("a_silent_answer_to_the_stream_shows_silence_not_a_stopped_watcher", async () => {
  app.link = running;
  stream({ kind: "silent" }, () => silent);
  const page = show(Now);

  await until(() => page.querySelector(".block.dim") !== null, "the faded blocks");

  expect(page.querySelector(".failure")).toBeNull();
});

test("a_window_opened_while_the_watcher_is_silent_has_no_values_and_says_so", async () => {
  app.link = silent;
  stream(null);
  const page = show(Now);

  await until(() => notes(page).length === 3, "the notices");

  expect(page.querySelector(".tbl")).toBeNull();
  expect(notes(page)[1]).toContain(t("ui.now.silent.nothing", { time: formatClock(app.answeredAt) }));
  expect(page.querySelector(".failure")).toBeNull();
});

test("a_silent_stream_of_a_watcher_that_answers_again_is_started_anew", async () => {
  app.link = running;
  const live = stream({ kind: "silent" });
  const page = show(Now);

  await until(() => live.starts() >= 2, "a second stream");

  expect(page.querySelector(".failure")).toBeNull();
});
