import { afterEach, beforeEach, expect, test } from "vitest";

import { agentStatusSamples } from "../dev/fixtures/wire";
import { answer } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { app } from "../lib/app.svelte";
import { historyReset } from "../lib/historyReset.svelte";
import { setLang, t } from "../lib/i18n.svelte";
import type { HistoryReset } from "../lib/types";
import HistoryResetDialog from "./HistoryResetDialog.svelte";

const FILE = "vestigium.db.corrupt-1790000000000";

/** A running watcher that set a damaged history aside, keeping `kept` of what the person had. */
function recovered(kept: Partial<HistoryReset> = {}, file = FILE): void {
  const base = agentStatusSamples[1];
  const reset: HistoryReset = { file, at_ms: Date.now(), tweaks: true, settings: true, machine: true, ...kept };
  app.link = { link: "running", ...base, monitor: { ...base.monitor, history_reset: reset } };
}

beforeEach(() => {
  localStorage.clear();
  historyReset.seen = null;
});

afterEach(() => {
  app.link = null;
  localStorage.clear();
  historyReset.seen = null;
});

function open(page: HTMLElement): HTMLDialogElement | null {
  const dialog = page.querySelector("dialog");
  return dialog?.open ? dialog : null;
}

test("the_first_opening_after_a_recovery_names_the_file_and_what_was_kept", async () => {
  recovered();
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  const text = open(page)!.textContent ?? "";
  expect(text).toContain(t("ui.history.reset.title"));
  expect(text).toContain(FILE);
  expect(text).not.toContain("\\");
  for (const item of ["tweaks", "settings", "machine"]) expect(text).toContain(t(`ui.history.reset.item.${item}`));
  expect(text.match(new RegExp(t("ui.history.reset.kept"), "g"))).toHaveLength(3);
  expect(text).not.toContain(t("ui.history.reset.lost"));
});

test("what_was_not_kept_is_listed_and_lost_tweaks_say_they_cannot_be_undone", async () => {
  recovered({ tweaks: false, machine: false });
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  const text = open(page)!.textContent ?? "";
  expect(text.match(new RegExp(t("ui.history.reset.not_kept"), "g"))).toHaveLength(2);
  expect(text.match(new RegExp(t("ui.history.reset.kept"), "g"))).toHaveLength(1);
  expect(text).toContain(t("ui.history.reset.lost"));
});

test("got_it_closes_it_and_the_same_file_does_not_ask_again", async () => {
  recovered();
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  const gotIt = [...page.querySelectorAll("button")].find((b) => b.textContent === t("ui.history.reset.got_it"));
  gotIt!.click();
  await until(() => open(page) === null, "the dialog to close");

  expect(localStorage.getItem("vestigium.history-reset.seen")).toBe(FILE);
  const again = show(HistoryResetDialog);
  await new Promise((resolve) => setTimeout(resolve, 20));
  expect(open(again)).toBeNull();
});

test("escape_closes_it_like_got_it", async () => {
  recovered();
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  open(page)!.dispatchEvent(new Event("cancel", { cancelable: true }));
  await until(() => open(page) === null, "the dialog to close");

  expect(historyReset.seen).toBe(FILE);
});

test("a_later_recovery_names_another_file_and_asks_again", async () => {
  historyReset.seen = FILE;
  recovered({}, "vestigium.db.corrupt-1790999999999");
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  expect(open(page)!.textContent).toContain("vestigium.db.corrupt-1790999999999");
});

test("a_watcher_that_set_nothing_aside_shows_no_dialog", async () => {
  const base = agentStatusSamples[1];
  app.link = { link: "running", ...base, monitor: { ...base.monitor, history_reset: null } };
  const page = show(HistoryResetDialog);
  await new Promise((resolve) => setTimeout(resolve, 20));

  expect(open(page)).toBeNull();
});

test("the_dialog_reads_in_portuguese", async () => {
  setLang("pt-BR");
  recovered();
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  const text = open(page)!.textContent ?? "";
  expect(text).toContain("O histórico recomeçou");
  expect(text).toContain("Entendi");
  expect(text).toContain(`Foi guardado como ${FILE} e um novo histórico foi iniciado.`);
});

test("open_folder_asks_the_window_to_open_the_data_folder", async () => {
  recovered();
  const calls = answer({ open_data_folder: () => null });
  const page = show(HistoryResetDialog);
  await until(() => open(page) !== null, "the dialog");

  [...page.querySelectorAll("button")].find((b) => b.textContent === t("ui.history.reset.open_folder"))!.click();
  await until(() => calls.of("open_data_folder").length === 1, "the command");
});
