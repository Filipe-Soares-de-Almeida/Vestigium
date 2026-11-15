import { flushSync } from "svelte";
import { beforeEach, expect, test } from "vitest";

import { startupCase } from "../dev/fixtures/startup";
import { answer, refusal } from "../dev/test-ipc";
import { show, until } from "../dev/test-mount";
import { bootTrace } from "../lib/bootTrace.svelte";
import { helperJobs } from "../lib/helper.svelte";
import { t } from "../lib/i18n.svelte";
import { startup } from "../lib/startup.svelte";
import { toast } from "../lib/toast.svelte";
import type { BootTraceCopied } from "../lib/types";
import StartupArea from "./health/StartupArea.svelte";
import StartupBoot from "./health/StartupBoot.svelte";

const k = (key: string) => t(`ui.health.startup.${key}`);
const FILE = String.raw`C:\Users\someone\AppData\Local\Vestigium\data\boot-traces\1790000000000.etl`;
const COPIED: BootTraceCopied = { path: FILE, exit_code: 0 };

type Copy = () => unknown;

beforeEach(() => {
  bootTrace.phase = "idle";
  bootTrace.path = null;
  helperJobs.last = {};
  helperJobs.asking = null;
  startup.dto = null;
  startup.error = null;
  toast.on = false;
});

function click(el: Element | null | undefined): void {
  (el as HTMLElement).click();
  flushSync();
}

/** The area with the trace row open; `copy` answers the copy command. */
async function area(copy: Copy = () => COPIED) {
  const calls = answer({ startup_area: () => startupCase("slow"), copy_boot_trace: copy, open_data_folder: () => null });
  const page = show(StartupArea, { finding: null, level: "Simple" });
  await until(() => page.querySelector(".lede") !== null, "the lede");
  const row = [...page.querySelectorAll<HTMLElement>("li")].find((li) => li.querySelector(".title")?.textContent === k("trace.title"));
  if (!row) throw new Error("no trace row");
  click(row.querySelector("button"));
  return { page, row, calls };
}

/** The page of one startup of the slow case: the latest, or the one `back` places before it. */
async function boot(back = 0, copy: Copy = () => COPIED) {
  const dto = startupCase("slow");
  const calls = answer({ startup_area: () => dto, copy_boot_trace: copy, open_data_folder: () => null });
  const start = dto.boots[dto.boots.length - 1 - back].kernel_start_ms;
  const page = show(StartupBoot, { boot: start, level: "Simple" });
  await until(() => page.querySelector("h1") !== null && page.querySelector(".loading") === null, "the title");
  const block = [...page.querySelectorAll<HTMLElement>("section.block")].find((s) => s.querySelector("h2")?.textContent === k("trace.block"));
  if (!block) throw new Error("no trace block");
  return { page, block, calls };
}

const buttons = (el: HTMLElement) => [...el.querySelectorAll<HTMLButtonElement>(".acts button")];
const copyButton = (el: HTMLElement) => buttons(el)[0];

test("the_area_offers_the_trace_as_a_second_action_with_the_admin_pill_and_no_button_until_opened", async () => {
  const calls = answer({ startup_area: () => startupCase("slow") });
  const page = show(StartupArea, { finding: null, level: "Simple" });
  await until(() => page.querySelector(".lede") !== null, "the lede");

  const rows = [...page.querySelectorAll<HTMLElement>("li")].filter((li) => li.querySelector(".title"));
  const titles = rows.map((li) => li.querySelector(".title")?.textContent);
  expect(titles.indexOf(k("trace.title"))).toBe(titles.indexOf(k("temp.title")) + 1);
  const row = rows[titles.indexOf(k("trace.title"))];
  expect(row.textContent).toContain(k("trace.sub"));
  expect(row.querySelector(".acts")).toBeNull();

  click(row.querySelector("button"));

  expect(row.textContent).toContain(k("trace.body"));
  expect(copyButton(row).textContent).toContain(k("trace.copy"));
  expect(row.textContent).toContain(t("ui.admin.required"));
  expect(calls.of("copy_boot_trace")).toHaveLength(0);
});

test("with_no_startup_read_the_area_has_no_trace_row", async () => {
  answer({ startup_area: () => startupCase("none") });
  const page = show(StartupArea, { finding: null, level: "Simple" });
  await until(() => page.querySelector(".lede") !== null, "the lede");

  expect(page.textContent).not.toContain(k("trace.title"));
});

test("a_copy_in_progress_disables_the_button_and_says_it_is_copying", async () => {
  let finish: (copied: BootTraceCopied) => void = () => {};
  const { row, calls } = await area(() => new Promise((resolve) => (finish = resolve)));

  click(copyButton(row));

  await until(() => calls.of("copy_boot_trace").length === 1, "the command");
  expect(copyButton(row).disabled).toBe(true);
  expect(copyButton(row).textContent).toContain(k("trace.running"));
  click(copyButton(row));
  expect(calls.of("copy_boot_trace")).toHaveLength(1);

  finish(COPIED);
  await until(() => bootTrace.phase === "done", "the copy");
  expect(copyButton(row).disabled).toBe(false);
});

test("a_copy_that_worked_shows_the_file_name_and_a_button_for_the_folder", async () => {
  const { row, calls } = await area();

  click(copyButton(row));
  await until(() => bootTrace.phase === "done", "the copy");

  expect(row.textContent).toContain(k("trace.done"));
  expect(row.textContent).toContain("1790000000000.etl");
  expect(row.textContent).not.toContain("boot-traces");
  const open = buttons(row).find((b) => b.textContent?.includes(k("trace.open")));
  expect(open).toBeDefined();
  click(open);
  await until(() => calls.of("open_data_folder").length === 1, "the command");
  expect(calls.of("open_data_folder")[0]).toEqual({ folder: "boot-traces" });
});

test("a_failed_folder_open_shows_a_toast", async () => {
  const { row } = await area();
  click(copyButton(row));
  await until(() => bootTrace.phase === "done", "the copy");
  answer({ open_data_folder: () => Promise.reject(refusal("failed")) });

  click(buttons(row).find((b) => b.textContent?.includes(k("trace.open"))));

  await until(() => toast.on, "the toast");
});

test("a_declined_permission_prompt_says_cancelled_and_is_not_an_error", async () => {
  const { row } = await area(() => Promise.reject(refusal("declined")));

  click(copyButton(row));
  await until(() => helperJobs.last.trace !== undefined, "the attempt");

  expect(row.textContent).toContain(t("ui.helper.cancelled.trace.title"));
  expect(row.textContent).toContain(t("ui.helper.cancelled.trace.body"));
  expect(row.textContent).not.toContain(k("trace.failed.title"));
  expect(row.querySelector(".status.attention")).toBeNull();
  expect(copyButton(row).disabled).toBe(false);
  expect(copyButton(row).textContent).toContain(k("trace.copy"));
});

test("a_trace_windows_does_not_have_has_its_own_sentence", async () => {
  const { row } = await area(() => ({ path: null, exit_code: 6 }));

  click(copyButton(row));
  await until(() => bootTrace.phase === "missing", "the answer");

  expect(row.textContent).toContain(k("trace.missing.title"));
  expect(row.textContent).toContain(k("trace.missing.body"));
  expect(row.textContent).not.toContain(k("trace.done"));
});

test("any_other_exit_code_or_error_is_a_failure_and_the_button_tries_again", async () => {
  const { row } = await area(() => ({ path: null, exit_code: 3 }));

  click(copyButton(row));
  await until(() => bootTrace.phase === "failed", "the answer");

  expect(row.textContent).toContain(k("trace.failed.title"));
  expect(row.textContent).toContain(k("trace.failed.body"));
  expect(copyButton(row).disabled).toBe(false);

  answer({ copy_boot_trace: () => Promise.reject(refusal("store")) });
  click(copyButton(row));
  await until(() => bootTrace.phase === "running", "the retry");
  await until(() => bootTrace.phase === "failed", "the answer");
});

test("a_helper_that_did_not_finish_in_time_asks_to_try_again_and_the_retry_copies", async () => {
  let attempts = 0;
  const { page, row } = await area(() => (attempts++ === 0 ? Promise.reject(refusal("helper_timed_out")) : COPIED));

  click(copyButton(row));
  await until(() => helperJobs.unseen("trace"), "the timeout");
  await until(() => page.querySelector("dialog[open]") !== null, "the dialog");

  expect(page.querySelector("dialog")?.textContent).toContain(t("ui.helper.timeout.body.trace"));
  click([...page.querySelectorAll<HTMLButtonElement>("dialog .acts button")].pop());
  await until(() => bootTrace.phase === "done", "the retry");
  expect(attempts).toBe(2);
});

test("the_page_of_the_latest_startup_has_a_trace_block_with_the_button", async () => {
  const { block, calls } = await boot();

  expect(block.textContent).toContain(k("trace.body"));
  expect(copyButton(block).textContent).toContain(k("trace.copy"));
  expect(block.textContent).toContain(t("ui.admin.required"));
  expect(block.textContent).not.toContain(k("trace.gone"));
  click(copyButton(block));
  await until(() => bootTrace.phase === "done", "the copy");
  expect(calls.of("copy_boot_trace")).toHaveLength(1);
  expect(block.textContent).toContain("1790000000000.etl");
});

test("the_page_of_an_older_startup_says_its_trace_is_gone_and_offers_no_copy", async () => {
  const { block, calls } = await boot(1);

  expect(block.textContent).toContain(k("trace.gone"));
  expect(block.querySelector("button")).toBeNull();
  expect(block.textContent).not.toContain(t("ui.admin.required"));
  expect(calls.of("copy_boot_trace")).toHaveLength(0);
});
