// Helper jobs that hang or get cancelled. URL: ?scan-hang=1 &dump-hang=1 (the read waits until it is cancelled
// or ends as "the helper did not finish in time" after <hang-ms>) &capture-late=1 (a capture ends as timed out)
// &trace=fail|missing|cancel|timeout (copying the boot trace fails, finds no trace, is declined at the permission
// prompt, or does not finish in time; any other value copies it after a second)
import type { BootTraceCopied } from "../../lib/types";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const cancelled = { scan: false, dumps: false };

/** Waits `ms`, or forever-ish while `hang` is set: ends as cancelled when asked, else as a timeout. */
export async function holdRead(read: "scan" | "dumps", ms: number, hang: boolean): Promise<void> {
  cancelled[read] = false;
  if (!hang) {
    await new Promise((resolve) => setTimeout(resolve, ms));
    return;
  }
  const until = Date.now() + Number(query.get("hang-ms") ?? 8000);
  while (Date.now() < until) {
    if (cancelled[read]) throw { kind: "cancelled", message: "cancelled" };
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw { kind: "helper_timed_out", message: "the helper did not finish in time" };
}

/** The folder the helper writes the copies of the boot trace to. */
const TRACE_FOLDER = String.raw`C:\Users\someone\AppData\Local\Vestigium\data\boot-traces`;

async function copyBootTrace(): Promise<BootTraceCopied> {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  const variant = query.get("trace");
  if (variant === "cancel") throw { kind: "declined", message: "the permission prompt was declined" };
  if (variant === "timeout") throw { kind: "helper_timed_out", message: "the helper did not finish in time" };
  if (variant === "missing") return { path: null, exit_code: 6 };
  if (variant === "fail") return { path: null, exit_code: 3 };
  return { path: `${TRACE_FOLDER}\\${Date.now()}.etl`, exit_code: 0 };
}

export const handlers: Record<string, Handler> = {
  copy_boot_trace: copyBootTrace,
  cancel_helper_read: (args) => {
    if (args.read === "scan" || args.read === "dumps") cancelled[args.read] = true;
    return null;
  },
};
