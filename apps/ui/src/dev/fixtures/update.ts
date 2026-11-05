// The in-app update in the browser. `?update=ask` asks the first-open question, `?update=found`
// finds a newer version, `?update=news` shows the notes after an update, `?update=offline` fails the check.
import type { Channel } from "@tauri-apps/api/core";

import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const mode = query.get("update");
const VERSION = "0.1.0";

const NOTES = [
  {
    version: "0.1.0",
    date: "2026-11-18",
    groups: [
      { kind: "Added", items: ["Holding Ctrl on the Now screen pauses it on the second it was pressed."] },
      { kind: "Fixed", items: ["Health, Dumps, Optimizations and Tests open with their data."] },
    ],
  },
  { version: "0.0.9", date: "2026-11-06", groups: [{ kind: "Fixed", items: ["Installing over a running copy replaces the watcher too."] }] },
];

let prefs = {
  check: mode === "ask" ? null : mode === "found" || mode === "offline",
  last_check_ms: null as number | null,
  seen_version: mode === "news" ? "0.0.8" : VERSION,
};
let timer: ReturnType<typeof setInterval> | undefined;

export const handlers: Record<string, Handler> = {
  "plugin:app|version": () => VERSION,
  update_prefs: () => prefs,
  set_update_prefs: (args) => void (prefs = args.prefs as typeof prefs),
  release_notes: () => NOTES,
  update_check: () => {
    if (mode === "offline") return Promise.reject({ kind: "failed", message: "offline" });
    if (mode !== "found") return { status: "up_to_date" };
    return {
      status: "available",
      version: "0.2.0",
      date_ms: Date.now(),
      groups: [
        { kind: "Added", items: ["A dialog says when a newer version is out, with what changed in it."] },
        { kind: "Changed", items: ["The side menu groups its screens by what you want to do."] },
      ],
    };
  },
  update_install: (args) => {
    const progress = args.progress as Channel<unknown>;
    const total = 5_787_924;
    let done = 0;
    clearInterval(timer);
    timer = setInterval(() => {
      done = Math.min(total, done + 420_000);
      progress.onmessage({ kind: "chunk", done, total });
      if (done >= total) {
        clearInterval(timer);
        progress.onmessage({ kind: "installing" });
      }
    }, 150);
    return null;
  },
  update_cancel: () => void clearInterval(timer),
};
