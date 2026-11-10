import { dayStart, itemsOf, worstOf } from "../screens/today/day";
import { marks } from "./api";
import type { OccurrenceSummary } from "./types";
import { pollWhileShown } from "./visibility";

const REFRESH_MS = 60_000;

/** Today's occurrences and overload periods, for the home cards and the navigation pane. */
class TodayMarks {
  occurrences = $state.raw<OccurrenceSummary[]>([]);
  periods = $state(0);

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
    } catch {
      // The counts keep their last values.
    }
  }
}

export const today = new TodayMarks();
