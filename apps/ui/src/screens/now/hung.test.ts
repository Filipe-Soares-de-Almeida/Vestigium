import assert from "node:assert/strict";
import { test } from "node:test";
import { HungTracker } from "./hung.ts";

const window = { pid: 7, name: "app.exe" };

test("a window hung without a break keeps its first start", () => {
  const tracker = new HungTracker();
  tracker.update([window], 10_000, 10_000, false);
  const next = tracker.update([window], 11_000, 11_000, false);
  assert.equal(next[0]?.sinceMs, 5_000);
  assert.equal(next[0]?.exact, true);
});

test("a hang that returns after a gap starts again from its new appearance", () => {
  const tracker = new HungTracker();
  tracker.update([window], 10_000, 10_000, false);
  const next = tracker.update([window], 60_000, 60_000, false);
  assert.equal(next[0]?.sinceMs, 55_000);
});

test("a gap of two ticks still counts as the same hang", () => {
  const tracker = new HungTracker();
  tracker.update([window], 10_000, 10_000, false);
  const next = tracker.update([window], 12_000, 12_000, false);
  assert.equal(next[0]?.sinceMs, 5_000);
});

test("a gap is measured by when the frames arrived, not by the chart clock", () => {
  const tracker = new HungTracker();
  tracker.update([window], 10_000, 10_000, false);
  const next = tracker.update([window], 10_500, 60_000, false);
  assert.equal(next[0]?.sinceMs, 5_500);
});

test("a window missing from a frame is forgotten", () => {
  const tracker = new HungTracker();
  tracker.update([window], 10_000, 10_000, false);
  tracker.update([], 11_000, 11_000, false);
  const next = tracker.update([window], 12_000, 12_000, false);
  assert.equal(next[0]?.sinceMs, 7_000);
});

test("a window listed in the first frame of a stream is only a bound", () => {
  const tracker = new HungTracker();
  const next = tracker.update([window], 10_000, 10_000, true);
  assert.equal(next[0]?.exact, false);
});
