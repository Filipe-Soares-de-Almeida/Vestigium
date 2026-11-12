// The in-app update in the browser. `?update=ask` asks the first-open question, `?update=found`
// finds a newer version, `?update=news` shows the notes after an update, `?update=offline` fails the check.
import type { Channel } from "@tauri-apps/api/core";

import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const mode = query.get("update");
const VERSION = "0.1.1";

const NOTES = [
  {
    version: "0.1.1",
    date: "2026-11-18",
    summary: null,
    groups: [
      { kind: "Added", items: ["Holding Ctrl on the Now screen pauses it on the second it was pressed."] },
      { kind: "Fixed", items: ["A freeze report opens on the part you clicked in the day's timeline."] },
    ],
  },
  {
    version: "0.1.0",
    date: "2026-11-11",
    summary: "The first release.",
    groups: [{ kind: "Added", items: ["The first release: freeze reports, crash dumps, driver captures, Health, Tests and Optimizations."] }],
  },
];

let prefs = {
  check: mode === "ask" ? null : mode === "found" || mode === "offline",
  last_check_ms: null as number | null,
  seen_version: mode === "news" ? "0.1.0" : VERSION,
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
      summary: null,
      groups: [
        { kind: "Changed", items: ["Reports explain storage stalls in simpler words."] },
        { kind: "Fixed", items: ["The day's timeline keeps its zoom when the day rolls over."] },
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
