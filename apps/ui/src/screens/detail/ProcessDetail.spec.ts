import { afterEach, expect, test } from "vitest";

import {
  agentLinkSamples,
  liveMessageSamples,
  livePointSamples,
  nowFrameSamples,
  processSampleSamples,
} from "../../dev/fixtures/wire";
import { answer } from "../../dev/test-ipc";
import { show, until } from "../../dev/test-mount";
import { app } from "../../lib/app.svelte";
import { t } from "../../lib/i18n.svelte";
import type { LiveMessage } from "../../lib/types";
import ProcessDetail from "./ProcessDetail.svelte";

const running = agentLinkSamples.find((link) => link.link === "running" && !link.paused)!;

afterEach(() => {
  app.link = null;
});

function watch(): { of: (cmd: string) => number } {
  const recent: LiveMessage = { kind: "recent", points: [...livePointSamples] };
  const now = liveMessageSamples[1] as Extract<LiveMessage, { kind: "now" }>;
  const frame = {
    ...nowFrameSamples[0],
    programs: { ts_ms: 0, entries: [{ ...processSampleSamples[0], name: "browser.exe" }] },
  };
  const calls = answer({
    live_start: (args) => {
      const channel = args.onFrame as { onmessage: (message: LiveMessage) => void };
      setTimeout(() => channel.onmessage(recent), 0);
      setTimeout(() => channel.onmessage({ kind: "now", point: now.point, frame }), 5);
      return 1;
    },
    live_stop: () => null,
    agent_status: () => running,
    program_recent: () => [],
    process_facts: () => null,
    program_marks: () => ({ history: "ready", occurrences: [], episodes: [], by_test: [], gaps: [], tests: [] }),
  });
  return { of: (cmd) => calls.of(cmd).length };
}

test("a_silent_monitor_fades_the_live_numbers_of_a_program_and_stops_asking_for_more", async () => {
  app.link = running;
  const calls = watch();
  const page = show(ProcessDetail, { name: "browser.exe" });
  await until(() => page.querySelector(".facts") !== null, "the program facts");
  expect(page.querySelector(".facts")?.classList.contains("veiled")).toBe(false);
  expect(page.querySelector(".silent-note")).toBeNull();

  const asked = calls.of("program_recent");
  app.link = { link: "silent", seconds: 3 };
  await until(() => page.querySelector(".silent-note") !== null, "the notice");

  expect(page.querySelector(".facts")?.classList.contains("veiled")).toBe(true);
  expect(page.textContent).toContain(t("ui.detail.process.today"));
  expect(page.querySelector(".failure")).toBeNull();
  expect(calls.of("program_recent")).toBe(asked);
});
