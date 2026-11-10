import type { HistoryScope } from "../../lib/router.svelte";
import type { MarksDto, OccurrenceSummary } from "../../lib/types";

// Days are local calendar days, named by "YYYY-MM-DD" keys.

const pad = (n: number) => String(n).padStart(2, "0");

export function dayKey(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Local midnight of a day key. */
export function dayMs(key: string): number {
  const [y, m, d] = key.split("-").map(Number);
  return new Date(y, m - 1, d).getTime();
}

export function addDays(key: string, n: number): string {
  const [y, m, d] = key.split("-").map(Number);
  return dayKey(new Date(y, m - 1, d + n).getTime());
}

/** 0 for Sunday. */
export const weekday = (key: string) => new Date(dayMs(key)).getDay();

export const monthOf = (key: string) => key.slice(0, 7);

export function monthStep(month: string, n: number): string {
  const [y, m] = month.split("-").map(Number);
  const total = y * 12 + (m - 1) + n;
  return `${Math.floor(total / 12)}-${pad((total % 12) + 1)}`;
}

export function monthRange(month: string): { from: string; to: string } {
  const [y, m] = month.split("-").map(Number);
  return { from: `${month}-01`, to: dayKey(new Date(y, m, 0).getTime()) };
}

export interface HistoryData {
  /** The oldest day anything was recorded on. */
  first: string;
  today: string;
  counts: Map<string, number>;
  byDay: Map<string, OccurrenceSummary[]>;
}

/** Counts the occurrences per day; the first day is the oldest mark of any kind, or today. */
export function buildHistory(now: number, marks: MarksDto): HistoryData {
  const today = dayKey(now);
  const counts = new Map<string, number>();
  const byDay = new Map<string, OccurrenceSummary[]>();
  let oldest = Infinity;
  for (const o of marks.occurrences) {
    const key = dayKey(o.start_ms);
    counts.set(key, (counts.get(key) ?? 0) + 1);
    const list = byDay.get(key);
    if (list) list.push(o);
    else byDay.set(key, [o]);
    if (o.start_ms < oldest) oldest = o.start_ms;
  }
  for (const list of byDay.values()) list.sort((a, b) => a.start_ms - b.start_ms);
  for (const e of marks.episodes) if (e.start_ms < oldest) oldest = e.start_ms;
  for (const g of marks.gaps) if (g.start_ms < oldest) oldest = g.start_ms;
  const first = oldest === Infinity ? today : dayKey(oldest);
  return { first, today, counts, byDay };
}

/** The days a period covers in a month: the whole month, or the last one or two weeks of it. */
export function rangeFor(data: HistoryData, month: string, scope: HistoryScope): { first: string; last: string } {
  const { from, to } = monthRange(month);
  const last = to > data.today ? data.today : to;
  const weeks = scope === "week" ? 1 : scope === "2w" ? 2 : 0;
  const start = weeks ? addDays(last, -weekday(last) - 7 * (weeks - 1)) : from;
  return { first: [start, from, data.first].sort().pop()!, last };
}

/** Days of the range, newest first. */
export function daysOf(range: { first: string; last: string }): string[] {
  const days: string[] = [];
  for (let d = range.last; d >= range.first; d = addDays(d, -1)) days.push(d);
  return days;
}

/** The month the page shows: the one asked for, kept between the first record and today. */
export function clampMonth(data: HistoryData, asked: string | null): string {
  const current = monthOf(data.today);
  const month = asked ?? current;
  return month > current ? current : month < monthOf(data.first) ? monthOf(data.first) : month;
}
