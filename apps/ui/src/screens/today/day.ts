import type {
  Episode,
  Gap,
  GapReason,
  MarksDto,
  OccurrenceKind,
  OccurrenceSummary,
  PcState,
  Severity,
  TimelineDto,
} from "../../lib/types";

export const HOUR_MS = 3_600_000;
const DAYS_SHOWN = 14;

/** Rows of the Estados do PC card, top to bottom. */
export const STATES: readonly PcState[] = ["pressure", "busy", "calm", "suspended", "no_reading"];

/** The token each state is drawn in; no reading is drawn hollow, in the ink of the text. */
export const STATE_TOKEN: Record<PcState, string> = {
  pressure: "press",
  busy: "busy",
  calm: "calm",
  suspended: "susp",
  no_reading: "ink2",
};

/**
 * The newest minutes of a day, whose recorded absence the watcher may not have written yet (it
 * writes in batches of up to five minutes): a stretch without a reading there is left empty.
 */
export const OPEN_TAIL_MS = 6 * 60_000;

/** Consecutive points in one state, `[from, to)` in Unix milliseconds. */
export interface Run {
  state: PcState;
  from: number;
  to: number;
}

export type Tone = "busy" | "press" | "freeze";

/** A row of the list: an occurrence, which has a report, or a pressure episode, which has none. */
export type Item =
  | { kind: "occurrence"; start: number; occurrence: OccurrenceSummary }
  | { kind: "episode"; start: number; episode: Episode; byTest: boolean };

export interface DayCount {
  start: number;
  count: number;
  today: boolean;
}

/** Everything the screen draws for one day, from local midnight to `now`. */
export interface Day {
  start: number;
  now: number;
  timeline: TimelineDto;
  runs: Run[];
  totals: Record<PcState, number>;
  occurrences: OccurrenceSummary[];
  episodes: Episode[];
  gaps: Gap[];
  items: Item[];
  days: DayCount[];
}

/** Local midnight of the day that holds `ms`, `offset` days later. */
export function dayStart(ms: number, offset = 0): number {
  const date = new Date(ms);
  date.setHours(0, 0, 0, 0);
  date.setDate(date.getDate() + offset);
  return date.getTime();
}

/** Where the 14 days start, so one `marks` call covers the chart and today. */
export function rangeStart(now: number): number {
  return dayStart(now, 1 - DAYS_SHOWN);
}

/** Stretches of the timeline between `start` and `end`. */
export function runsOf(timeline: TimelineDto, start: number, end: number): Run[] {
  const runs: Run[] = [];
  timeline.t.forEach((seconds, at) => {
    const from = Math.max(start, seconds * 1000);
    const to = Math.min(seconds * 1000 + timeline.step_ms, end);
    if (to <= from) return;
    const state = timeline.state[at];
    const last = runs.at(-1);
    if (last && last.state === state && last.to === from) last.to = to;
    else runs.push({ state, from, to });
  });
  return runs;
}

/**
 * Leaves the newest minutes without a reading empty; when the watcher is away (`away`), the stretch
 * that reaches `end` is an absence and becomes suspended.
 */
export function settleTail(runs: Run[], end: number, away: boolean, realNow = Date.now()): Run[] {
  const tail = realNow - OPEN_TAIL_MS;
  const settled: Run[] = [];
  for (const run of runs) {
    if (run.state !== "no_reading" || run.to <= tail) {
      settled.push(run);
      continue;
    }
    if (away && run.to >= end) {
      const before = settled.at(-1);
      if (before && before.state === "suspended" && before.to === run.from) before.to = run.to;
      else settled.push({ ...run, state: "suspended" });
    } else if (run.from < tail) {
      settled.push({ ...run, to: tail });
    }
  }
  return settled;
}

export function totalsOf(runs: Run[]): Record<PcState, number> {
  const totals: Record<PcState, number> = { pressure: 0, busy: 0, calm: 0, suspended: 0, no_reading: 0 };
  for (const run of runs) totals[run.state] += run.to - run.from;
  return totals;
}

/** Freezes and app hangs are the strongest mark; otherwise the tone follows severity. */
export function toneOf(occurrence: OccurrenceSummary): Tone {
  if (occurrence.severity === "Severe" || occurrence.kind === "AppHang") return "freeze";
  return occurrence.severity === "Medium" ? "press" : "busy";
}

const SEVERITY: Record<Severity, number> = { Light: 0, Medium: 1, Severe: 2 };
/** As the watcher ranks kinds: a freeze is gravest, then a hung app, a stutter, an event alone. */
const KIND: Record<OccurrenceKind, number> = { PostMortem: 4, Freeze: 3, AppHang: 2, Stutter: 1, Reported: 0 };

function gravity(o: OccurrenceSummary): number[] {
  return [SEVERITY[o.severity], KIND[o.kind], o.end_ms - o.start_ms];
}

function graver(a: OccurrenceSummary, b: OccurrenceSummary): boolean {
  const [x, y] = [gravity(a), gravity(b)];
  const at = x.findIndex((v, i) => v !== y[i]);
  return at >= 0 && x[at] > y[at];
}

/** The gravest occurrence: by severity, then kind, then duration. */
export function worstOf(occurrences: OccurrenceSummary[]): OccurrenceSummary | null {
  return occurrences.reduce<OccurrenceSummary | null>((worst, o) => (!worst || graver(o, worst) ? o : worst), null);
}

/** Episodes that put the PC under pressure; hints do not. */
export function isPressure(episode: Episode): boolean {
  return typeof episode.kind === "string" || "DiskBusy" in episode.kind;
}

/** The state the PC spent most of its watched time in, or `null` when nothing was watched. */
export function mostlyOf(totals: Record<PcState, number>): Exclude<PcState, "suspended" | "no_reading"> | null {
  const watched = (["calm", "busy", "pressure"] as const).filter((s) => totals[s] > 0);
  if (watched.length === 0) return null;
  return watched.reduce((a, b) => (totals[b] > totals[a] ? b : a));
}

/** Index of the point that holds `ms`, or -1. */
export function pointAt(timeline: TimelineDto, ms: number): number {
  if (timeline.t.length === 0) return -1;
  const at = Math.floor((ms - timeline.t[0] * 1000) / timeline.step_ms);
  return at >= 0 && at < timeline.t.length ? at : -1;
}

export function gapAt(gaps: Gap[], ms: number): GapReason | null {
  return gaps.find((g) => ms >= g.start_ms && ms < g.end_ms)?.reason ?? null;
}

/** "C:" from the disk instance name "0 C:". */
export function diskLabel(name: string): string {
  return name.replace(/^\d+\s+/, "") || name;
}

/** A key for an episode row: its kind (with the disk) and start, then its place in the list. */
export function keyOfEpisode(episode: Episode, index: number): string {
  const kind = typeof episode.kind === "string" ? episode.kind : JSON.stringify(episode.kind);
  return `e${episode.start_ms}-${kind}-${index}`;
}

/** A key for a row of the list: an occurrence by its id, an episode by its identity. */
export function keyOfItem(item: Item, index: number): string {
  return item.kind === "occurrence" ? `o${item.occurrence.id}-${index}` : keyOfEpisode(item.episode, index);
}

/** The day's occurrences and pressure episodes, and the rows they make, oldest first. */
export function itemsOf(start: number, now: number, marks: MarksDto): { occurrences: OccurrenceSummary[]; episodes: Episode[]; items: Item[] } {
  const occurrences = marks.occurrences
    .filter((o) => o.start_ms >= start && o.start_ms < now)
    .sort((a, b) => a.start_ms - b.start_ms);
  // A Vestigium test that ran then caused some episodes; the marks say which, in episode order.
  const byTest = new Set(marks.episodes.filter((_, i) => marks.by_test[i]));
  const episodes = marks.episodes
    .filter((e) => isPressure(e) && e.end_ms > start && e.start_ms < now)
    .sort((a, b) => a.start_ms - b.start_ms);
  const items: Item[] = [
    ...occurrences.map((occurrence) => ({ kind: "occurrence" as const, start: occurrence.start_ms, occurrence })),
    ...episodes.map((episode) => ({ kind: "episode" as const, start: episode.start_ms, episode, byTest: byTest.has(episode) })),
  ].sort((a, b) => a.start - b.start);
  return { occurrences, episodes, items };
}

/** Assembles the day from the timeline of today and the marks of the last 14 days; `away` is a watcher that is paused or stopped. */
export function buildDay(now: number, timeline: TimelineDto, marks: MarksDto, away = false): Day {
  const start = dayStart(now);
  const runs = settleTail(runsOf(timeline, start, now), now, away);
  const { occurrences, episodes, items } = itemsOf(start, now, marks);
  const days: DayCount[] = Array.from({ length: DAYS_SHOWN }, (_, i) => {
    const from = dayStart(now, i + 1 - DAYS_SHOWN);
    const to = dayStart(now, i + 2 - DAYS_SHOWN);
    return {
      start: from,
      count: marks.occurrences.filter((o) => o.start_ms >= from && o.start_ms < to).length,
      today: i === DAYS_SHOWN - 1,
    };
  });
  return {
    start,
    now,
    timeline,
    runs,
    totals: totalsOf(runs),
    occurrences,
    episodes,
    gaps: marks.gaps.filter((g) => g.end_ms > start),
    items,
    days,
  };
}
