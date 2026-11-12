import assert from "node:assert/strict";
import { test } from "node:test";
import type { Episode, MarksDto, OccurrenceSummary } from "../../lib/types";
import { itemsOf, keyOfEpisode, keyOfItem } from "./day.ts";

const START = Date.UTC(2026, 0, 5, 12, 0, 0);

function disk(name: string, start = START): Episode {
  return { kind: { DiskBusy: { disk: name } }, start_ms: start, end_ms: start + 60_000, peak: 100, culprit: null, truncated: false };
}

function cpu(start = START): Episode {
  return { kind: "CpuSaturated", start_ms: start, end_ms: start + 60_000, peak: 99, culprit: null, truncated: false };
}

function occurrence(id: number, start = START): OccurrenceSummary {
  return { id, kind: "Freeze", start_ms: start, end_ms: start + 5_000, severity: "Severe", cause: null } as OccurrenceSummary;
}

function marks(episodes: Episode[], occurrences: OccurrenceSummary[] = []): MarksDto {
  return {
    history: "ready",
    occurrences,
    episodes,
    by_test: episodes.map(() => false),
    gaps: [],
    tests: [],
  };
}

function keys(m: MarksDto): string[] {
  const { items } = itemsOf(START - 1000, START + 3_600_000, m);
  return items.map((item, i) => keyOfItem(item, i));
}

test("two disk episodes that start together have different keys", () => {
  const found = keys(marks([disk("0 C:"), disk("1 D:")]));
  assert.equal(found.length, 2);
  assert.equal(new Set(found).size, 2);
});

test("a processor and a disk episode that start together have different keys", () => {
  const found = keys(marks([cpu(), disk("0 C:")]));
  assert.equal(new Set(found).size, 2);
});

test("an occurrence and an episode that start together have different keys", () => {
  const found = keys(marks([cpu()], [occurrence(1)]));
  assert.equal(new Set(found).size, 2);
});

test("two occurrences that start together have different keys", () => {
  const found = keys(marks([], [occurrence(1), occurrence(2)]));
  assert.equal(new Set(found).size, 2);
});

test("identical episodes still get different keys", () => {
  const found = keys(marks([disk("0 C:"), disk("0 C:")]));
  assert.equal(new Set(found).size, 2);
});

test("an episode key names its kind and its disk", () => {
  assert.notEqual(keyOfEpisode(disk("0 C:"), 0), keyOfEpisode(disk("1 D:"), 0));
  assert.notEqual(keyOfEpisode(cpu(), 0), keyOfEpisode(disk("0 C:"), 0));
  assert.notEqual(keyOfEpisode(cpu(), 0), keyOfEpisode(cpu(), 1));
});
