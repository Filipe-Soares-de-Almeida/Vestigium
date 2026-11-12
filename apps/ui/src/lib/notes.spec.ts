import { expect, test } from "vitest";

import { setLang } from "./i18n.svelte";
import { CUT_TO, entryKey, highlights, parseEntryKey, plainNotes, releaseDay, shownOf, tagOf, wallOf } from "./notes";
import type { Release } from "./update.svelte";

const list = (kind: string, n: number) => Array.from({ length: n }, (_, i) => `${kind} ${i + 1}.`);

function release(counts: Record<string, number>, extra: Partial<Release> = {}): Release {
  return {
    version: "0.2.0",
    date: "2026-11-25",
    summary: null,
    groups: Object.entries(counts).map(([kind, n]) => ({ kind, items: list(kind, n) })),
    ...extra,
  };
}

const texts = (r: Release, max: number) => highlights(r, max).map((e) => e.text);

test("highlights_take_added_then_changed_and_end_with_the_first_security_entry", () => {
  const r = release({ Added: 3, Changed: 4, Fixed: 5, Security: 2 });
  expect(texts(r, 5)).toEqual(["Added 1.", "Added 2.", "Added 3.", "Changed 1.", "Security 1."]);
  expect(texts(r, 3)).toEqual(["Added 1.", "Added 2.", "Security 1."]);
});

test("highlights_fill_with_added_when_there_is_nothing_else_and_never_take_fixed", () => {
  expect(texts(release({ Added: 8 }), 5)).toEqual(["Added 1.", "Added 2.", "Added 3.", "Added 4.", "Added 5."]);
  expect(texts(release({ Fixed: 9, Security: 1 }), 5)).toEqual(["Security 1."]);
  expect(texts(release({ Fixed: 9 }), 5)).toEqual([]);
});

test("highlights_point_at_their_group_and_place", () => {
  const [first, , last] = highlights(release({ Changed: 2, Security: 3 }), 3);
  expect(entryKey(first)).toBe("Changed-0");
  expect(entryKey(last)).toBe("Security-0");
  expect(parseEntryKey("Fixed-12")).toEqual({ kind: "Fixed", index: 12 });
  expect(parseEntryKey("nonsense")).toBeNull();
  expect(parseEntryKey(null)).toBeNull();
});

test("a_group_of_more_than_15_shows_10", () => {
  expect(shownOf(15)).toBe(15);
  expect(shownOf(16)).toBe(CUT_TO);
  expect(shownOf(44)).toBe(10);
});

test("the_offered_version_tops_the_wall_once", () => {
  const notes = [release({ Added: 1 }, { version: "0.1.1" }), release({ Added: 1 }, { version: "0.1.0" })];
  const offered = release({ Added: 1 }, { version: "0.1.2" });
  expect(wallOf(notes, offered).map((r) => r.version)).toEqual(["0.1.2", "0.1.1", "0.1.0"]);
  expect(wallOf(notes, notes[0]).map((r) => r.version)).toEqual(["0.1.1", "0.1.0"]);
  expect(wallOf(notes, null)).toBe(notes);
});

test("the_tag_names_the_running_and_the_offered_versions", () => {
  expect(tagOf("0.1.1", "0.1.1", "0.1.2")).toBe("installed");
  expect(tagOf("0.1.2", "0.1.1", "0.1.2")).toBe("available");
  expect(tagOf("0.1.0", "0.1.1", null)).toBeNull();
});

test("copied_notes_are_plain_text_in_the_window_language", () => {
  setLang("pt-BR");
  const text = plainNotes(release({ Added: 1, Fixed: 2 }, { summary: "Uma versão curta." }));
  expect(text).toBe(
    "Vestigium 0.2.0 (25 de novembro de 2026)\n\nUma versão curta.\n\nNovo\n- Added 1.\n\nCorrigido\n- Fixed 1.\n- Fixed 2.\n",
  );
  setLang("en");
  expect(releaseDay("2026-11-01")).toBe("November 1, 2026");
  expect(releaseDay(null)).toBeNull();
});
