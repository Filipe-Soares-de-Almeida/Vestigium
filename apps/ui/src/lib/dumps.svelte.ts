import { deleteDumps, dumps as readDumps, startDumpScan } from "./api";
import { errorKind } from "./errors";
import { t, tn } from "./i18n.svelte";
import { toast } from "./toast.svelte";
import type { DumpsDto } from "./types";

const REFRESH_MS = 5 * 60_000;

/** Where a helper job stands; `running` covers Windows' permission prompt and the helper's run. */
export type HelperPhase = "idle" | "running" | "declined" | "failed" | "done";

/** What deleting removed: how many dumps and the space they took. */
export interface Freed {
  count: number;
  asked: number;
  bytes: number;
}

/** The dumps every screen reads (nav pane, post-mortem report, Dumps), reading them and deleting them. */
class Dumps {
  /** `null` until the first read. */
  dto = $state.raw<DumpsDto | null>(null);
  error = $state<unknown>(null);
  reading = $state<HelperPhase>("idle");
  deleting = $state<HelperPhase>("idle");
  /** Dumps ticked for deletion. */
  chosen = $state<number[]>([]);

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
      this.dto = await readDumps();
      this.error = null;
      const present = new Set(this.dto.dumps.filter((d) => d.deleted_ms === null).map((d) => d.id));
      this.chosen = this.chosen.filter((id) => present.has(id));
    } catch (error) {
      if (this.dto === null) this.error = error;
    }
  }

  choose(id: number, on: boolean): void {
    this.chosen = on ? [...new Set([...this.chosen, id])] : this.chosen.filter((x) => x !== id);
  }

  /** Ticks or clears every dump in `ids` at once. */
  chooseAll(ids: number[], on: boolean): void {
    this.chosen = on ? [...new Set([...this.chosen, ...ids])] : this.chosen.filter((x) => !ids.includes(x));
  }

  /** Asks Windows for permission, reads every dump in the helper and shows what they say. */
  async read(): Promise<void> {
    if (this.reading === "running") return;
    this.reading = "running";
    try {
      const done = await startDumpScan();
      if (done.exit_code !== 0) {
        this.reading = "failed";
        return;
      }
      await this.load();
      this.reading = "done";
      toast.show(done.dumps.length ? tn("ui.dumps.read.toast", done.dumps.length) : t("ui.dumps.read.toast.none"));
    } catch (error) {
      this.reading = errorKind(error) === "declined" ? "declined" : "failed";
    }
  }

  /** Deletes the ticked dumps in the helper; what each one said stays in the history. */
  async remove(): Promise<Freed | null> {
    const asked = this.chosen;
    if (!asked.length || this.deleting === "running") return null;
    this.deleting = "running";
    try {
      const done = await deleteDumps(asked);
      await this.load();
      const gone = this.dto?.dumps.filter((d) => asked.includes(d.id) && d.deleted_ms !== null) ?? [];
      if (done.exit_code !== 0 && !gone.length) {
        this.deleting = "failed";
        return null;
      }
      this.deleting = "done";
      return { count: gone.length, asked: asked.length, bytes: gone.reduce((sum, d) => sum + d.size_bytes, 0) };
    } catch (error) {
      this.deleting = errorKind(error) === "declined" ? "declined" : "failed";
      return null;
    }
  }
}

export const dumps = new Dumps();
