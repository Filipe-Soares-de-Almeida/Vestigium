import type { HungWindow } from "../../lib/types";

/** How long a window goes without input before Windows lists it as not responding. */
export const HUNG_AFTER_MS = 5_000;
/** A window missing from the frames for longer than this has recovered and hung again. */
const GAP_MS = 2_000;

/** A window not responding, and since when as far as this screen can tell. */
export interface Hung {
  pid: number;
  name: string;
  sinceMs: number;
  /** `false` when it was already listed as the stream began, so `sinceMs` is only a bound. */
  exact: boolean;
}

/** Follows the windows listed as not responding from one frame to the next. */
export class HungTracker {
  #items: Hung[] = [];
  #seenAt = new Map<number, number>();

  /**
   * The windows of this frame. A window keeps its start while it stays in the frames; after a
   * gap it starts again. `now` is the chart clock, `receivedAt` when the frame arrived.
   */
  update(windows: HungWindow[], now: number, receivedAt: number, fresh: boolean): Hung[] {
    const seenAt = new Map<number, number>();
    this.#items = windows.map((window) => {
      const known = this.#items.find((h) => h.pid === window.pid);
      const last = this.#seenAt.get(window.pid);
      seenAt.set(window.pid, receivedAt);
      if (known && last !== undefined && receivedAt - last <= GAP_MS) return known;
      return { pid: window.pid, name: window.name, sinceMs: now - HUNG_AFTER_MS, exact: !fresh };
    });
    this.#seenAt = seenAt;
    return this.#items;
  }
}
