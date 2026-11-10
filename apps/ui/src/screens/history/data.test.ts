import assert from "node:assert/strict";
import { test } from "node:test";
import type { Episode, MarksDto, OccurrenceSummary } from "../../lib/types";
import { buildHistory, dayKey } from "./data.ts";

const NOW = new Date(2026, 10, 10, 12, 0, 0).getTime();
const HOUR = 3_600_000;
const DAY = 24 * HOUR;

function occurrence(id: number, start: number): OccurrenceSummary {
  return { id, kind: "Freeze", start_ms: start, end_ms: start + 5_000, severity: "Severe", cause: null } as OccurrenceSummary;
}

function marks(occurrences: OccurrenceSummary[], episodes: Episode[] = []): MarksDto {
  return { history: "ready", occurrences, episodes, by_test: episodes.map(() => false), gaps: [], tests: [] };
}

test("occurrences are counted and listed per day, oldest first", () => {
  const data = buildHistory(NOW, marks([occurrence(3, NOW - HOUR), occurrence(1, NOW - DAY), occurrence(2, NOW - 2 * HOUR)]));
  assert.equal(data.counts.get(dayKey(NOW)), 2);
  assert.deepEqual(data.byDay.get(dayKey(NOW))?.map((o) => o.id), [2, 3]);
  assert.equal(data.counts.get(dayKey(NOW - DAY)), 1);
  assert.equal(data.first, dayKey(NOW - DAY));
  assert.equal(data.today, dayKey(NOW));
});

test("the first day is the oldest mark of any kind, or today when there is none", () => {
  const episode = { kind: "MemoryHigh", start_ms: NOW - 9 * DAY, end_ms: NOW - 9 * DAY + 60_000, peak: 95, culprit: null, truncated: false } as Episode;
  assert.equal(buildHistory(NOW, marks([occurrence(1, NOW - DAY)], [episode])).first, dayKey(NOW - 9 * DAY));
  assert.equal(buildHistory(NOW, marks([])).first, dayKey(NOW));
});

test("a very large history is summarised without overflowing the stack", () => {
  const many = Array.from({ length: 300_000 }, (_, i) => occurrence(i, NOW - (i % 3) * DAY - (i % 20) * 1_000));
  const data = buildHistory(NOW, marks(many));
  assert.equal(data.counts.get(dayKey(NOW)), 100_000);
  assert.equal(data.first, dayKey(NOW - 2 * DAY));
});
