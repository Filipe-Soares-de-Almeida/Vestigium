import { afterEach, expect, test } from "vitest";

import { tweakRowSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { pointCase } from "../screens/tweaks/words";
import { tweaks } from "./tweaks.svelte";
import type { RestorePoint, TweakRow, TweaksDto } from "./types";

const DROPPED = 0x2000_0005;
const pick = [{ tweak: "FastStartup", subject: null, value: null }] as never;

/** A history row of one change whose setting result is `error`, made with `point` standing behind it. */
function row(point: RestorePoint | null, error: number | null): TweakRow {
  const base = tweakRowSamples[1];
  return {
    ...base,
    id: 1,
    undone_ms: null,
    outside: false,
    change: { ...base.change, undo_of: null, results: base.change.results.slice(0, 1).map((r) => ({ ...r, error })), restore_point: point },
  };
}

/** The window's commands, with the history the watcher holds after the change. */
function serve(history: TweakRow[]) {
  return answer({
    apply_tweaks: () => ({ ids: [1], exit_code: 0 }),
    undo_tweak: () => ({ ids: [1], exit_code: 0 }),
    tweaks: () => ({ advice: [], history, last_restore_point_ms: null }) satisfies TweaksDto,
  });
}

afterEach(() => {
  tweaks.dto = null;
  tweaks.error = null;
});

test.each<[string, RestorePoint, string]>([
  ["off", "Off", "off"],
  ["a_failed_point", { Failed: { error: 5 } }, "failed"],
  ["a_point_logged_but_missing_from_the_list", { Failed: { error: DROPPED } }, "dropped"],
  ["an_unconfirmed_point", "Unconfirmed", "unconfirmed"],
])("apply_reports_why_no_point_was_made_for_%s", async (_name, point, expected) => {
  serve([row(point, 5)]);

  const outcome = await tweaks.apply(pick, false);

  expect(outcome.kind).toBe("no_point");
  if (outcome.kind !== "no_point") return;
  expect(outcome.point).toEqual(point);
  expect(pointCase(outcome.point)).toBe(expected);
});

test("undo_reports_why_no_point_was_made_too", async () => {
  serve([row({ Failed: { error: 1460 } }, 5)]);

  const outcome = await tweaks.undo(1, false);

  expect(outcome).toMatchObject({ kind: "no_point", point: { Failed: { error: 1460 } } });
});

test("a_change_that_was_written_is_done_whatever_its_point", async () => {
  for (const point of ["Created", "Reused", "Off", "Unconfirmed", { Failed: { error: 5 } }] as RestorePoint[]) {
    serve([row(point, null)]);
    expect((await tweaks.apply(pick, false)).kind).toBe("done");
  }
});

test("a_refusal_with_a_point_standing_is_not_blamed_on_the_point", async () => {
  for (const point of ["Created", "Reused", null] as (RestorePoint | null)[]) {
    serve([row(point, 5)]);
    expect((await tweaks.apply(pick, false)).kind).toBe("done");
  }
});

test("continuing_without_a_point_is_sent_to_the_helper_as_such", async () => {
  const calls = serve([row("Off", null)]);

  await tweaks.apply(pick, true);
  await tweaks.apply(pick, false);

  expect(calls.of("apply_tweaks").map((c) => c.withoutRestorePoint)).toEqual([true, false]);
});
