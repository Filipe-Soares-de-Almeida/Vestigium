import { afterEach, beforeEach, expect, test } from "vitest";

import { agentStatusSamples, tweakRowSamples, tweaksDtoSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { app } from "../lib/app.svelte";
import { t } from "../lib/i18n.svelte";
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
