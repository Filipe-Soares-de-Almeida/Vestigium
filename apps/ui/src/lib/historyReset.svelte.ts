import type { HistoryReset } from "./types";

const KEY = "vestigium.history-reset.seen";

/** The damaged history the person has already been told about, remembered by the name of the file set aside. */
class HistoryResetNotice {
  /** Name of the last file whose dialog was closed; `null` before any. */
  seen = $state<string | null>(recall());

  /** Whether the dialog for this set-aside history is still owed: a later recovery names another file. */
  owed(reset: HistoryReset | null | undefined): reset is HistoryReset {
    return reset !== null && reset !== undefined && reset.file !== this.seen;
  }

  see(reset: HistoryReset): void {
    this.seen = reset.file;
    try {
      localStorage.setItem(KEY, reset.file);
    } catch {
      // Without storage the dialog comes back at the next start of the window.
    }
  }
}

function recall(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export const historyReset = new HistoryResetNotice();
