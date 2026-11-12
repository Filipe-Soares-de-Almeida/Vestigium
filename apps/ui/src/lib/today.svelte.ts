import { dayStart, itemsOf, worstOf } from "../screens/today/day";
import { marks } from "./api";
import type { OccurrenceSummary } from "./types";
import { pollWhileShown } from "./visibility";

const REFRESH_MS = 60_000;

/** Today's occurrences and overload periods, for the home cards and the navigation pane. */
class TodayMarks {
  occurrences = $state.raw<OccurrenceSummary[]>([]);
  periods = $state(0);
  /** True once a read has succeeded; before that the empty lists above mean nothing was read. */
  loaded = $state(false);
  /** The reads so far all failed, so nothing is known; a failed refresh after a good read does not set it. */
  failed = $state(false);

  worst = $derived(worstOf(this.occurrences));

  /** Reads now and every minute while the window is visible; returns the function that stops it. */
  watch(): () => void {
    void this.load();
    return pollWhileShown(() => this.load(), REFRESH_MS);
  }

  /** A failed read keeps what is shown. */
  async load(): Promise<void> {
    const now = Date.now();
    const start = dayStart(now);
    try {
      const day = itemsOf(start, now, await marks(start, now));
      this.occurrences = day.occurrences;
      this.periods = day.episodes.length;
      this.loaded = true;
      this.failed = false;
    } catch {
      // The counts keep their last values; before the first good read they are unknown, not zero.
      if (!this.loaded) this.failed = true;
    }
  }
}

export const today = new TodayMarks();
