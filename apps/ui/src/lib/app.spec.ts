import { afterEach, expect, test } from "vitest";

import { agentLinkSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { app } from "./app.svelte";
import type { AgentLink } from "./types";

const running = agentLinkSamples.find((link) => link.link === "running" && !link.paused) as AgentLink;

afterEach(() => {
  app.link = null;
  app.history = null;
});

/** An answer the test settles by hand, so it can be held while the window asks again. */
function held<T>(): { promise: Promise<T>; settle: (value: T) => void } {
  let settle!: (value: T) => void;
  const promise = new Promise<T>((resolve) => (settle = resolve));
  return { promise, settle };
}

test("polls_never_overlap_and_a_call_during_one_asks_once_more", async () => {
  app.link = running;
  app.history = "ready";
  const gates = [held<AgentLink>(), held<AgentLink>()];
  const calls = answer({ agent_status: () => gates[calls.of("agent_status").length - 1].promise, settings: () => null });

  const first = app.refresh();
  const second = app.refresh();
  const third = app.refresh();
  await Promise.resolve();
  expect(calls.of("agent_status")).toHaveLength(1);

  gates[0].settle(running);
  await vi_tick();
  expect(calls.of("agent_status")).toHaveLength(2);
  gates[1].settle(running);
  await Promise.all([first, second, third]);

  expect(calls.of("agent_status")).toHaveLength(2);
});

test("a_silent_watcher_is_a_state_of_its_own_and_comes_back_when_it_answers", async () => {
  app.link = running;
  app.history = "ready";
  let link: AgentLink = { link: "silent", seconds: 3 };
  answer({ agent_status: () => link, settings: () => Promise.reject({ kind: "ipc", message: "silent" }) });

  await app.refresh();
  expect(app.silent).toBe(true);
  expect(app.silentSeconds).toBe(3);
  expect(app.unreachable).toBe(false);
  expect(app.settings).toBeNull();

  link = running;
  answer({ agent_status: () => link, settings: () => ({ settings: {}, autostart: true, editable: true }) });
  await app.refresh();
  expect(app.silent).toBe(false);
  expect(app.running).toBe(true);
});

test("a_busy_connection_leaves_what_the_window_knew", async () => {
  app.link = running;
  app.history = "ready";
  const calls = answer({ agent_status: () => ({ link: "busy" }) });

  await app.refresh();

  expect(app.link).toEqual(running);
  expect(calls.of("agent_status")).toHaveLength(2);
});

function vi_tick(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}
