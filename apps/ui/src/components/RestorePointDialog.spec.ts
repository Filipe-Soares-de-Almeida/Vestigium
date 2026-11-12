import { expect, test, vi } from "vitest";

import { show } from "../dev/test-mount";
import { setLang, t } from "../lib/i18n.svelte";
import type { RestorePoint } from "../lib/types";
import RestorePointDialog from "./RestorePointDialog.svelte";

const DROPPED = 0x2000_0005;

function open(point: RestorePoint, extra: { busy?: boolean; note?: string; attempt?: number } = {}) {
  const calls = { retry: vi.fn(), go: vi.fn(), cancel: vi.fn() };
  const page = show(RestorePointDialog, {
    point,
    attempt: 1,
    busy: false,
    note: "",
    ...extra,
    onretry: calls.retry,
    ongo: calls.go,
    oncancel: calls.cancel,
  });
  const dialog = page.querySelector("dialog")!;
  const steps = [...page.querySelectorAll("ol.trail li")].map((li) => ({
    name: li.querySelector(".nm")?.textContent,
    word: li.querySelector(".sx")?.textContent,
    mark: [...(li.querySelector(".status")?.classList ?? [])][1],
    here: li.classList.contains("here"),
  }));
  const button = (label: string) => [...page.querySelectorAll("button")].find((b) => b.textContent?.trim() === label)!;
  return { calls, dialog, steps, button, page };
}

const step = (key: string) => t(`ui.tweaks.no_point.step.${key}`);

test("system_restore_off_stops_at_the_request_and_checks_nothing_after_it", () => {
  const { dialog, steps } = open("Off");

  expect(dialog.open).toBe(true);
  expect(dialog.querySelector("h2")?.textContent).toBe(t("ui.tweaks.no_point.title.off"));
  expect(dialog.textContent).toContain(t("ui.tweaks.no_point.tail.off"));
  expect(steps.map((s) => s.word)).toEqual([step("not_done"), step("na"), step("unchecked")]);
  expect(steps.map((s) => s.here)).toEqual([true, false, false]);
});

test("a_failed_point_marks_the_step_with_the_windows_error_number", () => {
  const { dialog, steps } = open({ Failed: { error: 1460 } });

  expect(dialog.querySelector("h2")?.textContent).toBe(t("ui.tweaks.no_point.title.failed"));
  expect(steps[0].word).toBe(step("done"));
  expect(steps[1]).toMatchObject({ mark: "critical", here: true });
  expect(steps[1].word).toBe(`${step("failed")} · ${t("ui.tweaks.no_point.error", { code: "1460" })}`);
  expect(steps[2].word).toBe(step("unchecked"));
});

test("a_point_logged_as_created_but_missing_from_the_list_says_so_without_an_error_number", () => {
  const { dialog, steps } = open({ Failed: { error: DROPPED } });

  expect(dialog.querySelector("h2")?.textContent).toBe(t("ui.tweaks.no_point.title.unconfirmed"));
  expect(steps.map((s) => s.word)).toEqual([step("done"), step("logged"), step("missing")]);
  expect(steps.map((s) => s.here)).toEqual([false, false, true]);
  expect(dialog.textContent).not.toContain(String(DROPPED));
});

test("an_unconfirmed_point_does_not_claim_to_know_what_windows_did", () => {
  const { dialog, steps } = open("Unconfirmed");

  expect(dialog.querySelector("h2")?.textContent).toBe(t("ui.tweaks.no_point.title.unconfirmed"));
  expect(steps.map((s) => s.word)).toEqual([step("done"), step("unconfirmed"), step("unchecked")]);
  expect(steps[1]).toMatchObject({ mark: "attention", here: true });
});

test("a_windows_hresult_shows_in_hex", () => {
  const { steps } = open({ Failed: { error: 0x8004_1001 } });

  expect(steps[1].word).toContain("0x80041001");
});

test("the_three_buttons_call_back_and_try_again_has_its_own_accessible_name", () => {
  const { calls, button, page } = open("Unconfirmed");

  button(t("ui.tweaks.no_point.retry")).click();
  button(t("ui.tweaks.no_point.go")).click();
  button(t("ui.tweaks.cancel")).click();

  expect(calls.retry).toHaveBeenCalledTimes(1);
  expect(calls.go).toHaveBeenCalledTimes(1);
  expect(calls.cancel).toHaveBeenCalledTimes(1);
  expect(page.querySelector(`button[aria-label="${t("ui.tweaks.no_point.retry.label")}"]`)).not.toBeNull();
});

test("a_new_attempt_disables_every_button_and_ignores_escape", () => {
  const { calls, dialog, button, page } = open({ Failed: { error: 5 } }, { busy: true });

  expect(page.textContent).toContain(t("ui.tweaks.no_point.retrying"));
  for (const b of page.querySelectorAll("button")) expect(b.disabled).toBe(true);
  const escape = new Event("cancel", { cancelable: true });
  dialog.dispatchEvent(escape);
  expect(escape.defaultPrevented).toBe(true);
  expect(calls.cancel).not.toHaveBeenCalled();
  expect(button(t("ui.tweaks.no_point.retry")).disabled).toBe(true);
});

test("escape_gives_up_when_nothing_is_running", () => {
  const { calls, dialog } = open("Off");

  dialog.dispatchEvent(new Event("cancel", { cancelable: true }));

  expect(calls.cancel).toHaveBeenCalledTimes(1);
});

test("the_note_of_a_repeated_failure_is_announced", () => {
  const note = t("ui.tweaks.no_point.again.failed", { n: 2 });
  const { page } = open({ Failed: { error: 5 } }, { note, attempt: 2 });

  expect(page.querySelector("p[role=status]")?.textContent).toBe(note);
});

test("the_dialog_reads_in_portuguese", () => {
  setLang("pt-BR");
  const { dialog, steps } = open({ Failed: { error: 1460 } });

  expect(dialog.querySelector("h2")?.textContent).toBe("O Windows não criou o ponto de restauração");
  expect(steps[1].word).toBe("Falhou · Erro 1460 do Windows");
});
