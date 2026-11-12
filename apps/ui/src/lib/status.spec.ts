import { afterEach, expect, test } from "vitest";

import { agentStatusSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { app } from "./app.svelte";
import { t } from "./i18n.svelte";
import { shellStatus } from "./status";
import type { AgentLink, MonitorState } from "./types";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const base = agentStatusSamples[1];

/** The watcher's answer the way the agent builds it: the state is worked out from the age of the sample. */
function running(
  state: MonitorState,
  sampleAgeMs: number,
  rest: { paused?: boolean; suspended?: boolean; error?: string | null } = {},
): AgentLink {
  const now = Date.now();
  const stale = state === "no_reading";
  return {
    link: "running",
    ...base,
    started_at_ms: now - 5 * HOUR,
    last_sample_ms: now - sampleAgeMs,
    last_error: rest.error ?? null,
    paused: rest.paused ?? false,
    suspended: rest.suspended ?? false,
    monitor: {
      ...base.monitor,
      state,
      no_reading_since_ms: stale ? now - sampleAgeMs : null,
      resumed_ms: now - 5 * HOUR,
    },
  };
}

/** Puts a first answer in the app, then lets the poll bring in `next`, as the window does. */
async function poll(first: AgentLink, next: AgentLink): Promise<void> {
  app.link = first;
  app.history = "ready";
  answer({ agent_status: () => next });
  await app.refresh();
}

afterEach(() => {
  app.link = null;
  app.history = null;
});

test("a_stale_sample_with_an_error_reads_as_no_reading", async () => {
  await poll(running("watching", 1000), running("no_reading", 12 * MINUTE, { error: "counter query failed" }));

  const status = shellStatus();
  expect(status.tone).toBe("no_reading");
  expect(status.tone).not.toBe("ok");
  expect(status.text).not.toBe(t("ui.shell.status.watching"));
  expect(status.text).not.toContain(t("ui.shell.status.watching"));
  expect(status.text).not.toContain(t("ui.shell.status.paused"));
  expect(app.noReading).toBe(true);
});

test("a_paused_monitor_reads_as_paused_not_as_no_reading_even_with_an_old_sample", async () => {
  await poll(running("watching", 1000), running("watching", 3 * HOUR, { paused: true, error: "counter query failed" }));

  expect(shellStatus().tone).toBe("paused");
  expect(shellStatus().text).toBe(t("ui.shell.status.paused"));
  expect(app.noReading).toBe(false);
});

test("a_suspended_pc_with_an_old_sample_does_not_read_as_no_reading", async () => {
  await poll(running("watching", 1000), running("watching", 3 * HOUR, { suspended: true }));

  expect(shellStatus().tone).not.toBe("no_reading");
  expect(app.noReading).toBe(false);
});

test("a_fresh_sample_reads_as_watching", async () => {
  await poll(running("no_reading", 12 * MINUTE), running("watching", 2000));

  expect(shellStatus().tone).toBe("ok");
  expect(shellStatus().text).toContain(t("ui.shell.status.watching"));
  expect(app.noReading).toBe(false);
});

test("a_source_that_fails_while_samples_come_reads_as_partial_not_as_no_reading", async () => {
  const next = running("degraded", 2000, { error: "counter query failed" });
  await poll(running("watching", 1000), next);

  expect(shellStatus().tone).toBe("partial");
  expect(shellStatus().text).toBe(t("ui.shell.status.degraded"));
});
