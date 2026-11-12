// Helper jobs that hang or get cancelled. URL: ?scan-hang=1 &dump-hang=1 (the read waits until it is cancelled
// or ends as "the helper did not finish in time" after <hang-ms>) &capture-late=1 (a capture ends as timed out)
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

export const handlers: Record<string, Handler> = {
  cancel_helper_read: (args) => {
    cancelled[args.read as "scan" | "dumps"] = true;
    return null;
  },
};
