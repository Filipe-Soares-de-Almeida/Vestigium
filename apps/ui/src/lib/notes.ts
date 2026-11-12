import { lang, t, tn } from "./i18n.svelte";
import type { Release } from "./update.svelte";

// What the release notes pages show of a version: its highlights, its groups and its text to copy.

/** The headings of a version's notes, in the order Keep a Changelog writes them. */
export const KINDS = ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"] as const;

/** A group longer than this shows only its first `CUT_TO` entries until asked for the rest. */
export const CUT_OVER = 15;
export const CUT_TO = 10;

/** One entry of a version's notes and where it sits: its group and its place in it. */
export interface Entry {
  kind: string;
  index: number;
  text: string;
}

/** How a version stands on this PC: the one running, the one a check offers, or neither. */
export type Tag = "installed" | "available" | null;

/** A group's heading in the window's words; an unknown heading shows as written. */
export function heading(kind: string): string {
  return (KINDS as readonly string[]).includes(kind) ? t(`ui.update.group.${kind}`) : kind;
}

function entriesOf(release: Release, kind: string): Entry[] {
  return (release.groups.find((g) => g.kind === kind)?.items ?? []).map((text, index) => ({ kind, index, text }));
}

/** Up to `max` entries: Added first, then Changed, ending with the first Security entry when there is one. */
export function highlights(release: Release, max: number): Entry[] {
  const security = entriesOf(release, "Security").slice(0, 1);
  const room = Math.max(0, max - security.length);
  return [...entriesOf(release, "Added"), ...entriesOf(release, "Changed")].slice(0, room).concat(security);
}

/** How many entries of a group show before "Show the other N". */
export function shownOf(count: number): number {
  return count > CUT_OVER ? CUT_TO : count;
}

/** The address part that names an entry, as in `Fixed-3`. */
export function entryKey(entry: Pick<Entry, "kind" | "index">): string {
  return `${entry.kind}-${entry.index}`;
}

/** The entry an address names, or null when it names none. */
export function parseEntryKey(key: string | null): Pick<Entry, "kind" | "index"> | null {
  const found = /^([A-Za-z]+)-(\d+)$/.exec(key ?? "");
  return found ? { kind: found[1], index: Number(found[2]) } : null;
}

/** "11 November 2026" for a `YYYY-MM-DD` day, read as a calendar day. */
export function releaseDay(day: string | null): string | null {
  const found = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day ?? "");
  if (!found) return null;
  const ms = Date.UTC(Number(found[1]), Number(found[2]) - 1, Number(found[3]));
  return new Intl.DateTimeFormat(lang(), { day: "numeric", month: "long", year: "numeric", timeZone: "UTC" }).format(ms);
}

/** Every version newest first, with the one a check offers on top when the bundled notes do not have it. */
export function wallOf(notes: Release[], offered: Release | null): Release[] {
  return offered && !notes.some((r) => r.version === offered.version) ? [offered, ...notes] : notes;
}

/** The tag a version carries, from the version running and the one on offer. */
export function tagOf(version: string, installed: string, available: string | null): Tag {
  if (version === available) return "available";
  return version === installed ? "installed" : null;
}

/** A version's notes as plain text, with its date, summary and every group. */
export function plainNotes(release: Release): string {
  const day = releaseDay(release.date);
  const lines = [day ? `Vestigium ${release.version} (${day})` : `Vestigium ${release.version}`];
  if (release.summary) lines.push("", release.summary);
  for (const group of release.groups) {
    lines.push("", heading(group.kind), ...group.items.map((item) => `- ${item}`));
  }
  return `${lines.join("\n")}\n`;
}

/** "44 changes", in the window's language. */
export function changes(count: number): string {
  return tn("ui.news.changes", count);
}
