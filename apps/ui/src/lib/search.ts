import type { IconName } from "../components/Icon.svelte";
import { itemHref, type Group } from "./groups";
import { t } from "./i18n.svelte";
import { keyOf, tweaks } from "./tweaks.svelte";
import { href } from "./router.svelte";

export type SearchKind = "subjects" | "tweaks" | "settings";

export interface SearchEntry {
  kind: SearchKind;
  title: string;
  meta: string;
  href: string;
  icon: IconName;
  hay: string;
}

export interface SearchGroup {
  kind: SearchKind;
  items: SearchEntry[];
}

const KINDS: SearchKind[] = ["subjects", "tweaks", "settings"];
const PER_KIND = 6;

/** Lowercase without accents, so "memoria" finds "Memória". */
export const norm = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase();

/** Every page the search can open, from the groups and the tweaks the watcher advises on. */
export function buildIndex(groups: Group[]): SearchEntry[] {
  const out: SearchEntry[] = [];
  const add = (kind: SearchKind, title: string, meta: string, to: string, icon: IconName) =>
    out.push({ kind, title, meta, href: to, icon, hay: norm(`${title} ${meta}`) });
  for (const g of groups) {
    if (g.key === "settings") {
      for (const item of g.items) add("settings", item.label, g.name, itemHref(g, item), item.icon);
      continue;
    }
    add("subjects", g.name, g.sub, itemHref(g, g.items[0]), g.icon);
    for (const item of g.items.slice(1)) add("subjects", item.label, g.name, itemHref(g, item), item.icon);
  }
  for (const advice of tweaks.dto?.advice ?? []) {
    const title = t(`ui.tweaks.name.${advice.tweak}`, { name: advice.subject ?? "" });
    add("tweaks", title, t("ui.group.tweaks"), href.tweaks(keyOf(advice)), "tweaks");
  }
  return out;
}

/** Entries holding every typed word, titles first, grouped by kind. */
export function search(index: SearchEntry[], query: string): { words: string[]; groups: SearchGroup[] } {
  const words = norm(query).split(/\s+/).filter(Boolean);
  const groups: SearchGroup[] = [];
  for (const kind of KINDS) {
    const hits = index
      .map((entry, i) => ({ entry, i }))
      .filter(({ entry }) => entry.kind === kind && words.every((w) => entry.hay.includes(w)))
      .map(({ entry, i }) => ({ entry, i, rank: words.every((w) => norm(entry.title).includes(w)) ? 0 : 1 }))
      .sort((a, b) => a.rank - b.rank || a.i - b.i)
      .map(({ entry }) => entry);
    if (hits.length) groups.push({ kind, items: hits.slice(0, PER_KIND) });
  }
  return { words, groups };
}

/** Splits a title into runs, marking the letters the typed words match. */
export function highlight(text: string, words: string[]): { text: string; on: boolean }[] {
  const folded: string[] = [];
  const owner: number[] = [];
  [...text].forEach((ch, i) => {
    for (const c of norm(ch)) {
      folded.push(c);
      owner.push(i);
    }
  });
  const flat = folded.join("");
  const on = new Array<boolean>([...text].length).fill(false);
  for (const w of words) {
    for (let p = flat.indexOf(w); p >= 0 && w; p = flat.indexOf(w, p + w.length)) {
      for (let k = p; k < p + w.length; k++) on[owner[k]] = true;
    }
  }
  const runs: { text: string; on: boolean }[] = [];
  [...text].forEach((ch, i) => {
    const last = runs[runs.length - 1];
    if (last && last.on === on[i]) last.text += ch;
    else runs.push({ text: ch, on: on[i] });
  });
  return runs;
}
