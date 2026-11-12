// The in-app update in the browser. `?update=ask` asks the first-open question, `?update=found`
// finds a newer version, `?update=news` shows the notes after an update, `?update=offline` fails the check.
import type { Channel } from "@tauri-apps/api/core";

import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const mode = query.get("update");
const VERSION = "0.1.1";

// The bundled notes, as the window reads them from the changelogs.
const NOTES = [
  {
    version: "0.1.1",
    date: "2026-11-11",
    summary: null,
    groups: [
      {
        kind: "Added",
        items: [
          "Strip above every screen while any deep capture runs.",
          "Buttons to cancel captures, volume checks, dump reads and full scans.",
          "What was measured block and Coverage mark on a deep capture's page.",
          "Tray badges and window banners when the monitor is not reading the PC.",
          "Monitor not answering status, with the last values kept on screen.",
          "Dialog when the monitor set aside a damaged history file.",
        ],
      },
      {
        kind: "Changed",
        items: [
          "Install Vestigium for every account, in C:\\Program Files\\Vestigium.",
          "Keep the four cards on the home screen one height.",
          "Separate stretches with no reading from suspended ones in PC states.",
          "Open Dumps, History, Today and the charts faster with a long history.",
          "Keep the worst moments of every minute in a long deep capture.",
          "Show what a volume check marks for repair at the next restart.",
          "Compare a tweak's before and after per monitored hour.",
          "Put the probable cause and its confidence first in the copied report.",
          "Stop stating an uncertain cause as fact in reports, Now and Today.",
          "Stop presenting an interrupted shutdown as a power cut.",
          "Improve the text of Now, Today, Health and Optimizations.",
          "Update the README and the privacy, security and questions pages.",
        ],
      },
      {
        kind: "Fixed",
        items: [
          "Fix the window, a deep capture or a dump read waiting forever.",
          "Fix a second deep capture cutting the first one short.",
          "Fix a partial deep capture claiming no frames, drivers or stutters.",
          "Fix which drivers and apps a deep capture blames or leaves out.",
          "Fix reports blaming the wrong program or disk, or a nearby event.",
          "Fix generic next steps in blue screen and disk cause reports.",
          "Fix a report with no readings saying that nothing was overloaded.",
          "Fix occurrences lost, counted twice or shown whole when cut short.",
          "Fix storms of repeated Windows events flooding or splitting reports.",
          "Fix unexpected shutdowns lost, misreported or blamed on a power cut.",
          "Fix Today and All events failing when two overloads start at once.",
          "Fix Home and Health showing all clear for data they could not read.",
          "Fix Health alerts closing and reopening, or arriving an hour late.",
          "Fix false or stale disk alerts, and idle hard disks woken for a check.",
          "Fix failed checks reading as file system errors or a clean startup.",
          "Fix surface scans on a busy, failing or unplugged disk.",
          "Fix a running surface scan hidden after the window is reopened.",
          "Fix the speed test filling a small volume or leaving its file behind.",
          "Fix Windows Memory Diagnostic breaking History and other screens.",
          "Fix test and program pages left blank or silent on an error.",
          "Fix tweaks suggested on weak evidence or the app's own disk tests.",
          "Fix antivirus startup entries offered for removal.",
          "Fix Undo and failed tweaks leaving settings you did not choose.",
          "Show what an interrupted tweak changed, with Undo when it is known.",
          "Fix a tweak applied, or a wrong message, when a restore point failed.",
          "Fix Dumps missing subfolder dumps or reading another account's.",
          "Show why some crash dumps could not be deleted.",
          "Fix gaps in readings when the disk, history or Windows counters fail.",
          "Fix readings and events lost on sleep or a failed history write.",
          "Fix the monitor failing on a damaged or briefly unavailable history.",
          "Fix Delete the history losing unsaved data or leaving old items shown.",
          "Fix one bad retention setting resetting the other two.",
          "Fix the monitor piling up result files or memory without limit.",
          "Fix history deleted when the PC clock is set far ahead.",
          "Fix freezes and the Now chart thrown off by a PC clock change.",
          "Fix a pause or sensitive mode ending at the wrong time.",
          "Fix the window freezing on a busy monitor or losing it on a restart.",
          "Fix a second start of Vestigium running two monitors or closing silently.",
          "Fix the tray icon and menu freezing or showing an old state.",
          "Fix the monitor left running at logoff or during an install.",
          "Fix Start with Windows pointing at another copy of Vestigium.",
          "Fix the live view and the not-responding time on Now going stale.",
          "Fix the Programs list missing programs in search or their file path.",
          "Fix a failed update check saying Latest version installed.",
        ],
      },
      {
        kind: "Security",
        items: [
          "Refuse to start the administrator helper from a folder the account can change.",
          "Keep the update signing key out of the build of the installer.",
          "Refuse a tweak plan swapped while the administrator prompt is open.",
          "Refuse to run a library planted beside the administrator helper.",
          "Ignore links planted inside a crash dump folder.",
          "Stop a link from moving the helper's result file to another folder.",
          "Stop a folder swapped for a link from redirecting a helper job.",
          "Refuse an Undo that the administrator helper did not record.",
        ],
      },
    ],
  },
  {
    version: "0.1.0",
    date: "2026-11-09",
    summary: "The first release.",
    groups: [
      {
        kind: "Added",
        items: [
          "A light agent that monitors the PC from the tray, as your user and with no network connection.",
          "A report for every freeze, program that stopped responding, stutter and unexpected shutdown.",
          "A sensitive mode that checks the PC far more often for 30 minutes.",
          "Activity: the PC's states since midnight with every freeze and overload period marked.",
          "Health: the PC checked by area every hour, with alerts that show whether they grew.",
          "Tests: the Windows Memory Diagnostic, SMART, a surface scan and a speed test for each disk.",
        ],
      },
    ],
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
  release_notes: (args) => (args.after ? NOTES.filter((n) => n.version > String(args.after)) : NOTES),
  update_check: () => {
    if (mode === "offline") return Promise.reject({ kind: "failed", message: "offline" });
    if (mode !== "found") return { status: "up_to_date" };
    return {
      status: "available",
      version: "0.2.0",
      date_ms: Date.now(),
      summary: "Release notes get their own page, and the update dialog shows the highlights.",
      groups: [
        { kind: "Added", items: ["Release notes page with the highlights of each version."] },
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
