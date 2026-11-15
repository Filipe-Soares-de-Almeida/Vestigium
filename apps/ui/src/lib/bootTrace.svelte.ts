import { cancelHelperRead, copyBootTrace, openBootTraceFolder } from "./api";
import { errorKind, errorText } from "./errors";
import { helperJobs } from "./helper.svelte";
import { toast } from "./toast.svelte";

/** The helper's exit code when Windows has no boot trace to copy. */
export const TRACE_MISSING = 6;

/** Where copying the boot trace stands; `running` covers Windows' permission prompt and the helper's run. */
export type TracePhase = "idle" | "running" | "done" | "missing" | "failed";

/** The copy of the boot trace this window makes, shared by the startup area and the page of the latest startup. */
class BootTrace {
  phase = $state<TracePhase>("idle");
  /** The copy made, once done. */
  path = $state<string | null>(null);

  /** The name of the copy, without its folder. */
  fileName = $derived(this.path?.split(/[\\/]/).pop() ?? "");

  /** Asks Windows for permission and copies the trace Windows keeps of the last startup. */
  async copy(): Promise<void> {
    if (this.phase === "running") return;
    this.phase = "running";
    this.path = null;
    helperJobs.clear("trace");
    try {
      const done = await copyBootTrace();
      if (done.exit_code === 0 && done.path !== null) {
        this.path = done.path;
        this.phase = "done";
      } else {
        this.phase = done.exit_code === TRACE_MISSING ? "missing" : "failed";
      }
    } catch (error) {
      const kind = errorKind(error);
      if (kind === "declined" || kind === "cancelled") {
        helperJobs.record("trace", "cancelled");
        this.phase = "idle";
      } else if (kind === "helper_timed_out") {
        helperJobs.record("trace", "timeout");
        this.phase = "idle";
      } else {
        this.phase = "failed";
      }
    }
  }

  /** Stops waiting for the copy in progress; it then ends as cancelled. */
  async cancel(): Promise<void> {
    try {
      await cancelHelperRead("trace");
    } catch {
      // The copy ends by itself when the helper does.
    }
  }

  /** Opens the folder of the copies in File Explorer. */
  async openFolder(): Promise<void> {
    try {
      await openBootTraceFolder();
    } catch (error) {
      toast.show(errorText(error));
    }
  }
}

export const bootTrace = new BootTrace();
