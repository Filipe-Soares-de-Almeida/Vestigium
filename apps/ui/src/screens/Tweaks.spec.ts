import { afterEach, beforeEach, expect, test } from "vitest";

import { agentStatusSamples, tweakRowSamples, tweaksDtoSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { show, unmountAll, until } from "../dev/test-mount";
import { app } from "../lib/app.svelte";
import { setLang, t } from "../lib/i18n.svelte";
import { toast } from "../lib/toast.svelte";
import { tweaks } from "../lib/tweaks.svelte";
import type { RestorePoint, TweakRow, TweaksDto } from "../lib/types";
import Tweaks from "./Tweaks.svelte";

const advice = { ...tweaksDtoSamples[1].advice[0], applier: "Elevated" as const, subject: null, status: "Ready" as const };

/** A change the helper refused for lack of a restore point, or wrote once the point stood. */
function row(id: number, point: RestorePoint): TweakRow {
  const base = tweakRowSamples[1];
  const refused = point !== "Created";
  return {
    ...base,
    id,
    undone_ms: null,
    outside: false,
    change: {
      ...base.change,
      tweak: advice.tweak,
      subject: null,
      undo_of: null,
      results: advice.changes.slice(0, 1).map((c) => ({ change: c, error: refused ? 5 : null })),
      restore_point: point,
    },
  };
}

/** The window's commands, with `script` giving each apply attempt's restore point in turn. */
function serve(script: RestorePoint[], hold?: Promise<void>) {
  const history: TweakRow[] = [];
  const dto = (): TweaksDto => ({ advice: [advice], history, last_restore_point_ms: null });
  const calls = answer({
    tweaks: dto,
    apply_tweaks: async () => {
      const n = calls.of("apply_tweaks").length;
      if (n === 2 && hold) await hold;
      const made = row(n, script[n - 1]);
      history.unshift(made);
      return { ids: [made.id], exit_code: 0 };
    },
  });
  return calls;
}

const buttons = (page: HTMLElement) => [...page.querySelectorAll<HTMLButtonElement>("button")];
const named = (page: HTMLElement, label: string) => buttons(page).find((b) => b.textContent?.trim() === label);
const pointDialog = (page: HTMLElement) => page.querySelector<HTMLDialogElement>("dialog.point");

async function applyFromThePage(): Promise<HTMLElement> {
  await tweaks.load();
  const page = show(Tweaks, { key: "FastStartup", group: null });
  await until(() => named(page, t("ui.tweaks.apply_one")) !== undefined, "the Apply button");
  named(page, t("ui.tweaks.apply_one"))!.click();
  const question = () => page.querySelector<HTMLDialogElement>("dialog.confirm");
  await until(() => question()?.open === true, "the question");
  buttons(question()!).find((b) => b.classList.contains("accent"))!.click();
  return page;
}

beforeEach(() => {
  app.link = { link: "running", ...agentStatusSamples[1] };
  tweaks.dto = null;
});

afterEach(() => {
  app.link = null;
  tweaks.dto = null;
  toast.on = false;
});

test("a_failed_point_opens_the_path_dialog_and_try_again_keeps_it_open_until_it_works", async () => {
  let release = () => {};
  const hold = new Promise<void>((resolve) => (release = resolve));
  const calls = serve([{ Failed: { error: 1460 } }, { Failed: { error: 1460 } }, "Created"], hold);
  const page = await applyFromThePage();

  await until(() => pointDialog(page)?.open === true, "the restore point dialog");
  expect(pointDialog(page)!.textContent).toContain(t("ui.tweaks.no_point.error", { code: "1460" }));
  expect(calls.of("apply_tweaks").map((c) => c.withoutRestorePoint)).toEqual([false]);

  named(page, t("ui.tweaks.no_point.retry"))!.click();
  await until(() => calls.of("apply_tweaks").length === 2, "the second attempt");
  expect(pointDialog(page)?.open).toBe(true);
  expect(pointDialog(page)!.textContent).toContain(t("ui.tweaks.no_point.retrying"));
  expect(named(page, t("ui.tweaks.no_point.retry"))!.disabled).toBe(true);

  release();
  await until(() => pointDialog(page)?.textContent?.includes(t("ui.tweaks.no_point.again.failed", { n: 2 })) ?? false, "the repeated failure");
  expect(named(page, t("ui.tweaks.no_point.retry"))!.disabled).toBe(false);

  named(page, t("ui.tweaks.no_point.retry"))!.click();
  await until(() => pointDialog(page) === null, "the dialog to close");
  expect(calls.of("apply_tweaks").map((c) => c.withoutRestorePoint)).toEqual([false, false, false]);
  expect(toast.on).toBe(true);
});

test("try_again_never_asks_to_go_on_without_a_point_and_continue_does", async () => {
  const calls = serve([{ Failed: { error: 5 } }, "Created"]);
  const page = await applyFromThePage();
  await until(() => pointDialog(page)?.open === true, "the restore point dialog");

  named(page, t("ui.tweaks.no_point.go"))!.click();
  await until(() => calls.of("apply_tweaks").length === 2, "the second request");

  expect(calls.of("apply_tweaks").map((c) => c.withoutRestorePoint)).toEqual([false, true]);
  await until(() => pointDialog(page) === null, "the dialog to close");
});

test("cancel_leaves_everything_unchanged_and_says_so", async () => {
  const calls = serve(["Unconfirmed"]);
  const page = await applyFromThePage();
  await until(() => pointDialog(page)?.open === true, "the restore point dialog");
  expect(pointDialog(page)!.querySelector("h2")?.textContent).toBe(t("ui.tweaks.no_point.title.unconfirmed"));

  named(page, t("ui.tweaks.cancel"))!.click();
  await until(() => pointDialog(page) === null, "the dialog to close");

  expect(calls.of("apply_tweaks")).toHaveLength(1);
  expect(toast.message).toBe(t("ui.tweaks.toast.nothing"));
});

test("system_restore_off_shows_its_own_title", async () => {
  serve(["Off"]);
  const page = await applyFromThePage();
  await until(() => pointDialog(page)?.open === true, "the restore point dialog");

  expect(pointDialog(page)!.querySelector("h2")?.textContent).toBe(t("ui.tweaks.no_point.title.off"));
});

test("each_reason_a_point_is_missing_has_its_own_wording_and_only_off_says_system_restore_is_off", async () => {
  const wording = {
    off: { title: t("ui.tweaks.no_point.title.off"), tail: t("ui.tweaks.no_point.tail.off") },
    failed: { title: t("ui.tweaks.no_point.title.failed"), tail: t("ui.tweaks.no_point.tail") },
    unconfirmed: { title: t("ui.tweaks.no_point.title.unconfirmed"), tail: t("ui.tweaks.no_point.tail") },
  };
  const cases: [RestorePoint, keyof typeof wording][] = [
    ["Off", "off"],
    [{ Failed: { error: 5 } }, "failed"],
    [{ Failed: { error: 0x2000_0005 } }, "unconfirmed"],
    ["Unconfirmed", "unconfirmed"],
  ];
  for (const [point, shown] of cases) {
    serve([point]);
    tweaks.dto = null;
    const page = await applyFromThePage();
    await until(() => pointDialog(page)?.open === true, "the restore point dialog");
    const text = pointDialog(page)!.textContent ?? "";

    expect(pointDialog(page)!.querySelector("h2")?.textContent).toBe(wording[shown].title);
    expect(text).toContain(wording[shown].tail);
    for (const other of Object.keys(wording).filter((name) => name !== shown) as (keyof typeof wording)[]) {
      if (wording[other].title !== wording[shown].title) expect(text).not.toContain(wording[other].title);
    }
    if (shown !== "off") expect(text).not.toContain(wording.off.tail);
    expect(named(page, t("ui.tweaks.no_point.retry"))).toBeDefined();
    expect(named(page, t("ui.tweaks.no_point.go"))).toBeDefined();
    expect(named(page, t("ui.tweaks.cancel"))).toBeDefined();
    unmountAll();
  }
});

test("try_again_asks_again_with_a_point_and_a_new_unconfirmed_answer_keeps_its_own_note", async () => {
  const calls = serve(["Unconfirmed", "Unconfirmed"]);
  const page = await applyFromThePage();
  await until(() => pointDialog(page)?.open === true, "the restore point dialog");

  named(page, t("ui.tweaks.no_point.retry"))!.click();
  await until(() => calls.of("apply_tweaks").length === 2, "the second attempt");
  await until(() => pointDialog(page)?.textContent?.includes(t("ui.tweaks.no_point.again.unconfirmed", { n: 2 })) ?? false, "the repeated answer");

  expect(calls.of("apply_tweaks").map((c) => c.withoutRestorePoint)).toEqual([false, false]);
  expect(pointDialog(page)!.textContent).not.toContain(t("ui.tweaks.no_point.again.failed", { n: 2 }));
});

test("a_failed_point_reads_in_portuguese_in_the_dialog_the_screen_opens", async () => {
  serve([{ Failed: { error: 1460 } }]);
  const page = await applyFromThePage();
  await until(() => pointDialog(page)?.open === true, "the restore point dialog");
  setLang("pt-BR");
  await until(() => pointDialog(page)?.querySelector("h2")?.textContent === t("ui.tweaks.no_point.title.failed"), "the Portuguese title");

  expect(pointDialog(page)!.querySelector("h2")?.textContent).toBe("O Windows não criou o ponto de restauração");
  expect(pointDialog(page)!.textContent).toContain(t("ui.tweaks.no_point.error", { code: "1460" }));
  expect(pointDialog(page)!.textContent).not.toContain(t("ui.tweaks.no_point.tail.off"));
  expect(named(page, t("ui.tweaks.no_point.retry"))).toBeDefined();
});

const WRITTEN = 0x2000_0002;
const UNCHANGED = 0x2000_0003;
const UNKNOWN = 0x2000_0004;

/** A change whose helper stopped, with what the watcher found each setting holding. */
function stopped(id: number, found: number[]): TweakRow {
  const base = tweakRowSamples[1];
  return {
    ...base,
    id,
    undone_ms: null,
    outside: false,
    applier: "Elevated",
    change: {
      ...base.change,
      tweak: advice.tweak,
      subject: null,
      undo_of: null,
      results: advice.changes.map((c, i) => ({ change: c, error: found[i] })),
      restore_point: "Unconfirmed",
    },
  };
}

async function showStopped(found: number[]) {
  const calls = answer({
    tweaks: () => ({ advice: [advice], history: [stopped(7, found)], last_restore_point_ms: null }) satisfies TweaksDto,
    undo_tweak: () => ({ ids: [8], exit_code: 0 }),
  });
  await tweaks.load();
  const page = show(Tweaks, { key: "FastStartup", group: null });
  await until(() => page.querySelector(".hrow") !== null, "the history");
  return { calls, page };
}

test("a_stopped_change_with_some_settings_written_explains_itself_and_confirms_the_undo_in_the_card", async () => {
  const { calls, page } = await showStopped([WRITTEN, UNKNOWN]);

  expect(page.textContent).toContain(t("ui.tweaks.why.lead"));
  expect(page.textContent).toContain(t("ui.tweaks.why.mixed"));
  expect(page.textContent).toContain(t("ui.tweaks.why.mixed.undo"));
  expect(page.textContent).toContain(t("ui.tweaks.set.written"));
  expect(page.textContent).toContain(t("ui.tweaks.set.unknown"));
  expect(page.querySelector(".hrow")!.textContent).toContain(t("ui.tweaks.row.interrupted.part"));
  expect(page.querySelector(".hrow")!.textContent).not.toContain(t("ui.tweaks.row.refused"));

  named(page, t("ui.tweaks.undo"))!.click();
  await until(() => named(page, t("ui.tweaks.undo.interrupted.go")) !== undefined, "the inline question");
  expect(page.textContent).toContain(t("ui.tweaks.undo.interrupted.title"));
  expect(page.querySelector<HTMLDialogElement>("dialog.confirm")?.open).toBe(false);
  expect(calls.of("undo_tweak")).toHaveLength(0);

  named(page, t("ui.tweaks.undo.interrupted.go"))!.click();
  await until(() => calls.of("undo_tweak").length === 1, "the undo");
  expect(calls.of("undo_tweak")[0].id).toBe(7);
  await until(() => toast.message === `${t("ui.tweaks.toast.undone", { name: "Fast startup", value: "off" })} · ${t("ui.tweaks.toast.undone.left")}`, "the toast");
});

test("cancelling_the_inline_question_changes_nothing", async () => {
  const { calls, page } = await showStopped([WRITTEN, WRITTEN]);

  named(page, t("ui.tweaks.undo"))!.click();
  await until(() => named(page, t("ui.tweaks.cancel")) !== undefined, "the inline question");
  expect(page.textContent).toContain(t("ui.tweaks.undo.title"));
  named(page, t("ui.tweaks.cancel"))!.click();
  await until(() => named(page, t("ui.tweaks.cancel")) === undefined, "the question to close");

  expect(named(page, t("ui.tweaks.undo"))).toBeDefined();
  expect(calls.of("undo_tweak")).toHaveLength(0);
});

test("a_stopped_change_with_an_unknown_outcome_offers_no_undo_and_does_not_claim_nothing_was_written", async () => {
  const { page } = await showStopped([UNKNOWN, UNKNOWN]);

  expect(page.textContent).toContain(t("ui.tweaks.why.unknown"));
  expect(page.textContent).toContain(t("ui.tweaks.fact.unconfirmed"));
  expect(page.querySelector(".hrow")!.textContent).toContain(t("ui.tweaks.row.interrupted.unknown"));
  expect(page.textContent).not.toContain(t("ui.tweaks.row.refused"));
  expect(named(page, t("ui.tweaks.undo"))).toBeUndefined();
  expect(named(page, t("ui.tweaks.apply_one"))).toBeDefined();
});

test("a_stopped_change_that_left_everything_as_it_was_says_so_and_offers_apply", async () => {
  const { page } = await showStopped([UNCHANGED, UNCHANGED]);

  expect(page.textContent).toContain(t("ui.tweaks.why.none"));
  expect(page.querySelector(".hrow")!.textContent).toContain(t("ui.tweaks.row.interrupted.none"));
  expect(named(page, t("ui.tweaks.undo"))).toBeUndefined();
  expect(named(page, t("ui.tweaks.apply_one"))).toBeDefined();
});
