import assert from "node:assert/strict";
import { test } from "node:test";
import type { ProcessSample } from "../../lib/types";
import { visiblePrograms } from "./sort.ts";

function program(n: number): ProcessSample {
  return {
    pid: n,
    name: `program-${n}.exe`,
    cpu_pct: 200 - n,
    private_bytes: n * 1000,
    read_bps: 0,
    write_bps: 0,
  } as ProcessSample;
}

const list = Array.from({ length: 150 }, (_, i) => program(i + 1));

test("a program ranked 120th by cpu is found by name", () => {
  const { rows, matched } = visiblePrograms(list, "cpu", "program-120.exe", 100);
  assert.deepEqual(rows.map((p) => p.pid), [120]);
  assert.equal(matched, 1);
});

test("an empty query returns the first 100 in order", () => {
  const { rows, matched } = visiblePrograms(list, "cpu", "  ", 100);
  assert.equal(rows.length, 100);
  assert.equal(rows[0].pid, 1);
  assert.equal(rows[99].pid, 100);
  assert.equal(matched, 150);
});

test("sorting by memory ranks the matches of a query", () => {
  const { rows, matched } = visiblePrograms(list, "memory", "PROGRAM-14", 100);
  assert.equal(matched, 11);
  assert.equal(rows[0].pid, 149);
  assert.equal(rows[10].pid, 14);
});

test("a query with many matches is cut to the limit but counted whole", () => {
  const { rows, matched } = visiblePrograms(list, "cpu", "program", 20);
  assert.equal(rows.length, 20);
  assert.equal(matched, 150);
});
