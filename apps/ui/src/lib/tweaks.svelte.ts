import { applyTweaks, tweaks as readTweaks, undoTweak } from "./api";
import { errorKind } from "./errors";
import type { RestorePoint, TweakPick, TweakRow, TweaksDto } from "./types";
import { pollWhileShown } from "./visibility";

const REFRESH_MS = 5 * 60_000;

/** How a request to change Windows ended. */
export type Outcome =
  | { kind: "done"; rows: TweakRow[] }
  /** Windows made no restore point, as `point` says; nothing that needed one was written. */
  | { kind: "no_point"; rows: TweakRow[]; point: RestorePoint }
  | { kind: "declined" }
  | { kind: "failed"; error: unknown };

/** The tweaks every screen reads (nav pane, Health, Dumps, Optimizations), applying and undoing them. */
class Tweaks {
  /** `null` until the first read. */
  dto = $state.raw<TweaksDto | null>(null);
  error = $state<unknown>(null);
  /** A change is under way; Windows may be asking for permission. */
  busy = $state(false);

  recommended = $derived(this.dto?.advice.filter((a) => a.status === "Ready" && a.signals.length > 0).length ?? 0);

  /** Reads now and every five minutes while the window is visible; returns the function that stops it. */
  watch(): () => void {
    void this.load();
    return pollWhileShown(() => this.load(), REFRESH_MS);
  }

  /** A failed read keeps what is on screen; the error shows only when there is nothing yet. */
  async load(): Promise<void> {
    try {
      this.dto = await readTweaks();
      this.error = null;
    } catch (error) {
      if (this.dto === null) this.error = error;
    }
  }

  apply(picks: TweakPick[], withoutRestorePoint: boolean): Promise<Outcome> {
    return this.#run(() => applyTweaks(picks, withoutRestorePoint));
  }

  undo(id: number, withoutRestorePoint: boolean): Promise<Outcome> {
    return this.#run(() => undoTweak(id, withoutRestorePoint));
  }

  async #run(change: () => Promise<{ ids: number[] }>): Promise<Outcome> {
    if (this.busy) return { kind: "failed", error: null };
    this.busy = true;
    try {
      const { ids } = await change();
      await this.load();
      const rows = this.dto?.history.filter((r) => ids.includes(r.id)) ?? [];
      const point = blockedPoint(rows);
      return point ? { kind: "no_point", rows, point } : { kind: "done", rows };
    } catch (error) {
      return errorKind(error) === "declined" ? { kind: "declined" } : { kind: "failed", error };
    } finally {
      this.busy = false;
    }
  }
}

/** The restore point that kept a refused row from being written, if one did. */
function blockedPoint(rows: TweakRow[]): RestorePoint | null {
  for (const r of rows) {
    const p = r.change.restore_point;
    const refused = r.change.results.some((x) => x.error !== null);
    if (refused && p !== null && (p === "Off" || p === "Unconfirmed" || (typeof p === "object" && "Failed" in p))) return p;
  }
  return null;
}

/** One tweak's address: its name, and the startup entry for one of those. */
export function keyOf(t: { tweak: string; subject: string | null }): string {
  return t.subject ? `${t.tweak}:${t.subject}` : t.tweak;
}

export const tweaks = new Tweaks();
