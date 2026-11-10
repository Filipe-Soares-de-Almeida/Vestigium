import assert from "node:assert/strict";
import { afterEach, beforeEach, mock, test } from "node:test";

const page = Object.assign(new EventTarget(), { hidden: false });
Object.assign(globalThis, { document: page });

const { isShown, pollWhileShown, setMinimized, whileVisible } = await import("./visibility.ts");

const hide = (hidden: boolean) => {
  page.hidden = hidden;
  page.dispatchEvent(new Event("visibilitychange"));
};

beforeEach(() => {
  mock.timers.enable({ apis: ["setInterval", "Date"] });
});

afterEach(() => {
  mock.timers.reset();
  page.hidden = false;
  setMinimized(false);
});

test("the window counts as shown unless the page is hidden or the window is minimized", () => {
  const stop = pollWhileShown(() => {}, 1_000);
  assert.equal(isShown(), true);
  setMinimized(true);
  assert.equal(isShown(), false);
  setMinimized(false);
  hide(true);
  assert.equal(isShown(), false);
  stop();
});

test("a poll runs on its interval while shown and not while minimized", () => {
  let runs = 0;
  const stop = pollWhileShown(() => runs++, 1_000);
  mock.timers.tick(3_000);
  assert.equal(runs, 3);
  setMinimized(true);
  mock.timers.tick(60_000);
  assert.equal(runs, 3);
  stop();
});

test("a poll runs on a hidden page only after it is shown again", () => {
  let runs = 0;
  const stop = pollWhileShown(() => runs++, 1_000);
  hide(true);
  mock.timers.tick(10_000);
  assert.equal(runs, 0);
  hide(false);
  assert.equal(runs, 1, "a full interval passed, so it reads at once");
  stop();
});

test("restoring a minimized window reads at once when an interval has passed, not before", () => {
  let runs = 0;
  const stop = pollWhileShown(() => runs++, 5_000);
  mock.timers.tick(5_000);
  assert.equal(runs, 1);
  setMinimized(true);
  mock.timers.tick(2_000);
  setMinimized(false);
  assert.equal(runs, 1, "less than an interval since the last read");
  mock.timers.tick(3_000);
  assert.equal(runs, 2, "the next tick reads");
  setMinimized(true);
  mock.timers.tick(20_000);
  setMinimized(false);
  assert.equal(runs, 3);
  stop();
});

test("a stopped poll never runs again", () => {
  let runs = 0;
  const stop = pollWhileShown(() => runs++, 1_000);
  stop();
  mock.timers.tick(5_000);
  setMinimized(true);
  setMinimized(false);
  assert.equal(runs, 0);
});

test("whileVisible starts and stops with the window being shown", async () => {
  const log: string[] = [];
  const stop = whileVisible(async () => {
    log.push("start");
    return () => log.push("stop");
  });
  await new Promise((resolve) => setImmediate(resolve));
  setMinimized(true);
  await new Promise((resolve) => setImmediate(resolve));
  setMinimized(false);
  await new Promise((resolve) => setImmediate(resolve));
  stop();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(log, ["start", "stop", "start", "stop"]);
});
