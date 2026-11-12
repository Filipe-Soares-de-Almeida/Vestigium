import { afterEach, beforeEach, expect, test } from "vitest";

import { show, until } from "../dev/test-mount";
import { helperJobs } from "../lib/helper.svelte";
import { setLang, t } from "../lib/i18n.svelte";
import HelperDialogs from "./HelperDialogs.svelte";
import HelperLine from "./HelperLine.svelte";

beforeEach(() => {
  helperJobs.last = {};
  helperJobs.asking = null;
});

afterEach(() => {
  helperJobs.last = {};
  helperJobs.asking = null;
});

const open = (page: HTMLElement) => {
  const dialog = page.querySelector("dialog");
  return dialog?.open ? dialog : null;
};
const button = (page: HTMLElement, label: string) =>
  [...page.querySelectorAll("button")].find((b) => b.textContent?.trim() === label);

test("cancelling_a_capture_asks_first_and_says_what_is_lost", async () => {
  let cancelled = 0;
  const page = show(HelperDialogs, { job: "capture", onconfirm: () => (cancelled += 1) });
  expect(open(page)).toBeNull();

  helperJobs.ask("capture");
  await until(() => open(page) !== null, "the question");

  const text = open(page)!.textContent ?? "";
  expect(text).toContain(t("ui.helper.ask.capture.title"));
  expect(text).toContain(t("ui.helper.ask.capture.body"));
  expect(t("ui.helper.ask.capture.body")).toContain("will not be saved");
  expect(cancelled).toBe(0);

  button(page, t("ui.helper.ask.capture.yes"))!.click();
  await until(() => open(page) === null, "the dialog to close");
  expect(cancelled).toBe(1);
  expect(helperJobs.asking).toBeNull();
});

test("keeping_the_job_closes_the_question_and_cancels_nothing", async () => {
  let cancelled = 0;
  const page = show(HelperDialogs, { job: "check", onconfirm: () => (cancelled += 1) });
  helperJobs.ask("check");
  await until(() => open(page) !== null, "the question");

  button(page, t("ui.helper.ask.check.no"))!.click();
  await until(() => open(page) === null, "the dialog to close");
  expect(cancelled).toBe(0);
});

test("escape_keeps_the_job_running", async () => {
  let cancelled = 0;
  const page = show(HelperDialogs, { job: "capture", onconfirm: () => (cancelled += 1) });
  helperJobs.ask("capture");
  await until(() => open(page) !== null, "the question");

  open(page)!.dispatchEvent(new Event("cancel", { cancelable: true }));
  await until(() => open(page) === null, "the dialog to close");
  expect(cancelled).toBe(0);
});

test("a_job_that_cancels_directly_never_asks", async () => {
  const page = show(HelperDialogs, { job: "dumps", onretry: () => {} });
  helperJobs.ask("dumps");
  await new Promise((resolve) => setTimeout(resolve, 20));
  expect(open(page)).toBeNull();
});

test("a_helper_that_did_not_finish_is_told_once_on_its_screen", async () => {
  let retried = 0;
  const page = show(HelperDialogs, { job: "dumps", onretry: () => (retried += 1) });
  helperJobs.record("dumps", "timeout");
  await until(() => open(page) !== null, "the dialog");

  const text = open(page)!.textContent ?? "";
  expect(text).toContain(t("ui.helper.timeout.title"));
  expect(text).toContain(t("ui.helper.timeout.body.dumps"));

  button(page, t("ui.helper.timeout.close"))!.click();
  await until(() => open(page) === null, "the dialog to close");
  expect(helperJobs.unseen("dumps")).toBe(false);
  expect(helperJobs.last.dumps?.kind).toBe("timeout");
  expect(retried).toBe(0);

  const again = show(HelperDialogs, { job: "dumps", onretry: () => {} });
  await new Promise((resolve) => setTimeout(resolve, 20));
  expect(open(again)).toBeNull();
});

test("try_again_closes_the_dialog_and_starts_the_job", async () => {
  let retried = 0;
  const page = show(HelperDialogs, { job: "health", onretry: () => (retried += 1) });
  helperJobs.record("health", "timeout");
  await until(() => open(page) !== null, "the dialog");

  button(page, t("ui.common.retry"))!.click();
  await until(() => open(page) === null, "the dialog to close");
  expect(retried).toBe(1);
});

test("a_capture_that_did_not_finish_does_not_say_nothing_was_lost", () => {
  expect(t("ui.helper.timeout.body.capture")).not.toMatch(/nothing was lost/i);
  expect(t("ui.helper.timeout.body.dumps")).toMatch(/nothing was lost/i);
});

test("a_cancel_is_a_line_on_the_card_and_never_a_dialog", async () => {
  helperJobs.record("capture", "cancelled");
  const line = show(HelperLine, { job: "capture" });
  const dialogs = show(HelperDialogs, { job: "capture", onconfirm: () => {}, onretry: () => {} });
  await new Promise((resolve) => setTimeout(resolve, 20));

  expect(line.textContent).toContain(t("ui.helper.cancelled.capture.title"));
  expect(line.textContent).toContain(t("ui.helper.cancelled.capture.body"));
  expect(open(dialogs)).toBeNull();
});

test("the_card_keeps_a_line_about_the_last_attempt_that_timed_out", () => {
  helperJobs.record("health", "timeout");
  helperJobs.see("health");
  const line = show(HelperLine, { job: "health" });

  expect(line.textContent).toContain(t("ui.helper.timeout.last"));
  helperJobs.clear("health");
});

test("a_check_line_shows_only_on_the_volume_it_was_about", () => {
  helperJobs.record("check", "cancelled", "C:\\");

  expect(show(HelperLine, { job: "check", volume: "C:\\" }).textContent).toContain(t("ui.helper.cancelled.check.title"));
  expect(show(HelperLine, { job: "check", volume: "D:\\" }).textContent?.trim()).toBe("");
});

test("the_texts_are_translated_and_never_promise_to_end_the_helper", () => {
  const english = ["ui.helper.timeout.title", "ui.helper.ask.capture.body", "ui.helper.cancelled.capture.body"].map((key) => t(key));
  setLang("pt-BR");
  const portuguese = ["ui.helper.timeout.title", "ui.helper.ask.capture.body", "ui.helper.cancelled.capture.body"].map((key) => t(key));
  setLang("en");
  portuguese.forEach((text, i) => {
    expect(text).not.toBe(english[i]);
    expect(text).not.toMatch(/^ui\./);
  });
  for (const job of ["capture", "check"]) {
    expect(t(`ui.helper.ask.${job}.body`)).toContain("asked to stop");
    expect(t(`ui.helper.ask.${job}.body`)).toContain("stops waiting for it");
  }
});
