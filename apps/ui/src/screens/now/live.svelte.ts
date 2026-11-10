import type { LiveMessage, LivePoint, NowFrame } from "../../lib/types";

/** How much the chart shows. */
export const SPAN_MS = 600_000;
/** How long a window goes without input before Windows lists it as not responding. */
const HUNG_AFTER_MS = 5_000;

/** A window not responding, and since when as far as this screen can tell. */
export interface Hung {
  pid: number;
  name: string;
  sinceMs: number;
  /** `false` when it was already listed as the stream began, so `sinceMs` is only a bound. */
  exact: boolean;
}

/** What the live stream has brought so far: the last ten minutes, the latest second, the hung windows. */
export class Live {
  points = $state.raw<LivePoint[]>([]);
  /** The latest second; `null` before the first sample. */
  point = $state.raw<LivePoint | null>(null);
  frame = $state.raw<NowFrame | null>(null);
  hung = $state.raw<Hung[]>([]);
  nowMs = $state(0);
  /** The ten minutes arrived. */
  started = $state(false);
  /** The watcher stopped answering. */
  ended = $state(false);

  #fresh = true;

  /** Forgets everything, before a stream starts again. */
  reset(): void {
    this.started = false;
    this.ended = false;
  }

  take(message: LiveMessage): void {
    if (message.kind === "recent") {
      this.points = message.points;
      this.point = message.points.at(-1) ?? null;
      this.#fresh = true;
      this.started = true;
      this.ended = false;
      return;
    }
    if (message.kind === "stopped") {
      this.ended = true;
      return;
    }
    const now = message.point?.reading.ts_ms ?? Date.now();
    this.nowMs = now;
    this.frame = message.frame;
    if (message.point) {
      this.point = message.point;
      const last = this.points.at(-1);
      if (!last || message.point.reading.ts_ms !== last.reading.ts_ms) {
        // A point older than the last one means the clock stepped back: what is ahead of it goes.
        const from = now - SPAN_MS;
        this.points = [
          ...this.points.filter(
            (p) => p.reading.ts_ms >= from && p.reading.ts_ms < message.point!.reading.ts_ms,
          ),
          message.point,
        ];
      }
    }
    this.hung = message.frame.hung.map((window) => {
      const known = this.hung.find((h) => h.pid === window.pid);
      return (
        known ?? {
          pid: window.pid,
          name: window.name,
          sinceMs: now - HUNG_AFTER_MS,
          exact: !this.#fresh,
        }
      );
    });
    this.#fresh = false;
  }
}
