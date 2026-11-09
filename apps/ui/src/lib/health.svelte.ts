import { health as readHealth, startScan } from "./api";
import { errorKind } from "./errors";
import { t, tn } from "./i18n.svelte";
import { toast } from "./toast.svelte";
import type { FindingDto, HealthDto } from "./types";

const REFRESH_MS = 5 * 60_000;
/** The watcher evaluates a stored scan on its next tick; the page reads again after it. */
const AFTER_SCAN_MS = 2500;

/** Where the full scan stands; `running` covers Windows' permission prompt and the helper's run. */
export type ScanPhase = "idle" | "running" | "declined" | "failed" | "done";

/** The health findings every screen reads (nav pane, Today, Health) and the full scan this window runs. */
class Health {
  /** `null` until the first read. */
  dto = $state.raw<HealthDto | null>(null);
  error = $state<unknown>(null);
  phase = $state<ScanPhase>("idle");
  /** Findings the last scan opened, shown once it is done. */
  found = $state(0);

  critical = $derived(this.dto?.open.filter((f) => f.concern === "Critical").length ?? 0);
  attention = $derived(this.dto?.open.filter((f) => f.concern === "Attention").length ?? 0);

  /** Reads now and every five minutes while the window is visible; returns the function that stops it. */
  watch(): () => void {
    void this.load();
    const timer = setInterval(() => {
      if (!document.hidden) void this.load();
    }, REFRESH_MS);
    return () => clearInterval(timer);
  }

  /** A failed read keeps what is on screen; the error shows only when there is nothing yet. */
  async load(): Promise<void> {
    try {
      this.dto = await readHealth();
      this.error = null;
    } catch (error) {
      if (this.dto === null) this.error = error;
    }
  }

  /** Asks Windows for permission, runs the scan in the helper and reads the findings it changed. */
  async scan(): Promise<void> {
    if (this.phase === "running") return;
    this.phase = "running";
    const before = new Set(this.dto?.open.map(key) ?? []);
    try {
      const done = await startScan();
      if (done.exit_code !== 0 || done.scans.length === 0) {
        this.phase = "failed";
        return;
      }
      await this.load();
      await new Promise((resolve) => setTimeout(resolve, AFTER_SCAN_MS));
      await this.load();
      this.found = this.dto?.open.filter((f) => !before.has(key(f))).length ?? 0;
      this.phase = "done";
      toast.show(this.found ? tn("ui.health.scan.toast.found", this.found) : t("ui.health.scan.toast.none"));
    } catch (error) {
      this.phase = errorKind(error) === "declined" ? "declined" : "failed";
    }
  }
}

/** The same finding across reads: its kind and what it is about. */
function key(f: FindingDto): string {
  return `${f.kind}:${JSON.stringify(f.subject)}`;
}

export const health = new Health();
