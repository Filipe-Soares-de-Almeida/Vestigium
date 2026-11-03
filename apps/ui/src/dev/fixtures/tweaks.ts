// Tweaks page data. URL: ?tweaks=some|none|applied (default some) &restore=off|skipped &tweak-ms=<wait>
// Every name, path and value here is invented; nothing was read from a real PC.
import type {
  Advice,
  RestorePoint,
  SettingChange,
  TweakChange,
  TweakId,
  TweakPick,
  TweakRow,
  TweakSetting,
  TweaksApplied,
  TweaksDto,
  TweakValue,
} from "../../lib/types";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const DAY = 24 * 60 * 60_000;
const now = Date.now();

const LM = "LocalMachine" as const;
const CU = "CurrentUser" as const;
const dword = (hive: typeof LM | typeof CU, key: string, name: string): TweakSetting => ({ Dword: { hive, key, name } });
const change = (setting: TweakSetting, from: TweakValue, to: TweakValue): SettingChange => ({ setting, from, to });

const fastStartup = dword(LM, String.raw`SYSTEM\CurrentControlSet\Control\Session Manager\Power`, "HiberbootEnabled");
const dvr = [
  dword(CU, String.raw`Software\Microsoft\Windows\CurrentVersion\GameDVR`, "AppCaptureEnabled"),
  dword(CU, String.raw`Software\Microsoft\Windows\CurrentVersion\GameDVR`, "HistoricalCaptureEnabled"),
  dword(CU, String.raw`System\GameConfigStore`, "GameDVR_Enabled"),
];
const sysMain: TweakSetting = { Service: { name: "SysMain" } };
const search: TweakSetting = { Service: { name: "WSearch" } };
const running = { Service: { start: 2, delayed: false, running: true } };
const stopped = { Service: { start: 4, delayed: false, running: false } };
const localDumps = String.raw`SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps`;
const approval = (hive: typeof LM | typeof CU, name: string): TweakSetting => ({
  StartupApproval: { hive, list: "Run", name },
});
const disabled: TweakValue = { Bytes: [3, 0, 0, 0, 0, 80, 140, 120, 18, 77, 219, 1] };

function advice(
  tweak: TweakId,
  partial: Partial<Advice> & Pick<Advice, "changes">,
): Advice {
  return {
    tweak,
    subject: null,
    status: "Ready",
    signals: [],
    applier: "Elevated",
    takes: "Now",
    current: partial.changes.map((c) => ({ setting: c.setting, value: c.from })),
    ...partial,
  };
}

const ALL: Advice[] = [
  advice("FastStartup", {
    signals: [{ Finding: "ShutdownUnfinished" }, { UnfinishedShutdowns: { count: 4 } }],
    changes: [change(fastStartup, { Dword: 1 }, { Dword: 0 })],
    takes: "NextShutdown",
  }),
  advice("GameDvr", {
    signals: [{ Stutters: { count: 6 } }, { CaptureFrames: { start_ms: now - 2 * DAY, app: "Game.exe" } }],
    changes: dvr.map((s) => change(s, { Dword: 1 }, { Dword: 0 })),
    applier: "User",
  }),
  advice("SysMain", {
    signals: [{ Finding: "WindowsOnHdd" }, { DiskBusy: { count: 5, culprit: "svchost.exe" } }],
    changes: [change(sysMain, running, stopped)],
  }),
  advice("StartupApp", {
    subject: "ChatClient",
    signals: [{ SlowStartup: { name: "Chat Client", total_ms: 9200 } }],
    changes: [change(approval(CU, "ChatClient"), "Absent", disabled)],
    applier: "User",
    takes: "NextSignIn",
  }),
  advice("AppDumps", {
    signals: [{ AppCrashes: { count: 2 } }],
    changes: [
      change({ Key: { hive: LM, key: localDumps } }, "Absent", "Present"),
      change(dword(LM, localDumps, "DumpType"), "Absent", { Dword: 1 }),
      change(dword(LM, localDumps, "DumpCount"), "Absent", { Dword: 10 }),
    ],
  }),
  advice("PcieAspm", {
    changes: [
      change(
        { PowerAc: { subgroup: "501a4d13-42af-4429-9fd1-a8218c268e20", setting: "ee12f906-d277-404b-b6da-e5fa1a576df5" } },
        { Index: 1 },
        { Index: 0 },
      ),
    ],
    applier: "User",
  }),
  advice("UsbSuspend", {
    changes: [
      change(
        { PowerAc: { subgroup: "2a737441-1930-4402-8d77-b2bebba308a3", setting: "48e6b7a6-50f5-4782-a5d4-53bb8f07e226" } },
        { Index: 1 },
        { Index: 0 },
      ),
    ],
    applier: "User",
  }),
  advice("Hibernation", { changes: [change("Hibernation", { On: true }, { On: false })] }),
  advice("WindowsSearch", { changes: [change(search, { Service: { start: 2, delayed: true, running: true } }, stopped)] }),
  advice("GpuScheduling", {
    changes: [change(dword(LM, String.raw`SYSTEM\CurrentControlSet\Control\GraphicsDrivers`, "HwSchMode"), { Dword: 2 }, { Dword: 1 })],
    takes: "Restart",
  }),
  advice("Mpo", {
    changes: [change(dword(LM, String.raw`SOFTWARE\Microsoft\Windows\Dwm`, "OverlayTestMode"), "Absent", { Dword: 5 })],
    takes: "Restart",
  }),
  advice("StartupApp", {
    subject: "NotesSync",
    changes: [change(approval(CU, "NotesSync"), "Absent", disabled)],
    applier: "User",
    takes: "NextSignIn",
  }),
  advice("PowerPlan", {
    status: "AlreadySet",
    changes: [],
    current: [{ setting: "ActiveScheme", value: { Scheme: "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c" } }],
    applier: "User",
  }),
  advice("CrashDump", {
    status: "AlreadySet",
    changes: [],
    current: [{ setting: dword(LM, String.raw`SYSTEM\CurrentControlSet\Control\CrashControl`, "CrashDumpEnabled"), value: { Dword: 7 } }],
    takes: "Restart",
  }),
  advice("StartupApp", {
    subject: "SecurityTray",
    status: "Protected",
    changes: [],
    current: [{ setting: approval(LM, "SecurityTray"), value: "Absent" }],
    takes: "NextSignIn",
  }),
];

function initial(): { advice: Advice[]; history: TweakRow[] } {
  const level = query.get("tweaks") ?? "some";
  if (level === "none") {
    return {
      advice: ALL.map((a) => ({ ...a, signals: [] })),
      history: [],
    };
  }
  if (level === "applied") {
    const [first, dvrAdvice] = ALL;
    const row = (id: number, a: Advice, days: number, undone: number | null): TweakRow => ({
      id,
      applied_ms: now - days * DAY,
      undone_ms: undone === null ? null : now - undone * DAY,
      applier: a.applier,
      change: {
        tweak: a.tweak,
        subject: a.subject,
        undo_of: null,
        results: a.changes.map((c) => ({ change: c, error: null })),
        restore_point: a.applier === "Elevated" ? { Reused: { at_ms: now - (days + 1) * DAY } } : null,
      },
      helped: { days: Math.min(days, 7), before: 9, after: 2 },
      outside: false,
    });
    return {
      advice: ALL.filter((a) => a.tweak !== "GameDvr").concat(settledBy(dvrAdvice)),
      history: [row(2, dvrAdvice, 5, null), row(1, first, 12, 9)],
    };
  }
  return { advice: ALL, history: [] };
}

/** The advice once its changes are written: nothing left to change, the new values in place. */
function settledBy(a: Advice): Advice {
  return { ...a, status: "AlreadySet", changes: [], current: a.changes.map((c) => ({ setting: c.setting, value: c.to })) };
}

let state = initial();
let nextId = 10;

function point(): RestorePoint {
  const restore = query.get("restore");
  if (restore === "off") return "Off";
  if (restore === "skipped") return { Reused: { at_ms: now - 20 * 3_600_000 } };
  return "Created";
}

function record(a: Advice, without: boolean, undoOf: number | null = null): TweakRow {
  const p = a.applier === "Elevated" ? point() : null;
  const blocked = p === "Off" && !without;
  const change: TweakChange = {
    tweak: a.tweak,
    subject: a.subject,
    undo_of: undoOf,
    results: (blocked ? a.changes.slice(0, 1) : a.changes).map((c) => ({ change: c, error: blocked ? 5 : null })),
    restore_point: p,
  };
  return { id: nextId++, applied_ms: Date.now(), undone_ms: null, applier: a.applier, change, helped: null, outside: false };
}

const wait = () => new Promise((resolve) => setTimeout(resolve, Number(query.get("tweak-ms") ?? 1200)));

export const handlers: Record<string, Handler> = {
  tweaks: () =>
    ({
      advice: state.advice,
      history: state.history,
      last_restore_point_ms: now - 20 * 3_600_000,
    }) satisfies TweaksDto,
  apply_tweaks: async (args) => {
    await wait();
    const picks = args.picks as TweakPick[];
    const without = Boolean(args.withoutRestorePoint);
    const rows: TweakRow[] = [];
    for (const pick of picks) {
      const a = state.advice.find((x) => x.tweak === pick.tweak && x.subject === pick.subject && x.status === "Ready");
      if (!a) continue;
      const row = record(a, without);
      rows.push(row);
      if (row.change.results.every((r) => r.error === null)) {
        state.advice = state.advice.map((x) => (x === a ? settledBy(x) : x));
      }
    }
    state.history = [...rows, ...state.history];
    return { ids: rows.map((r) => r.id), exit_code: rows.some((r) => r.applier === "Elevated") ? 0 : null } satisfies TweaksApplied;
  },
  undo_tweak: async (args) => {
    await wait();
    const id = args.id as number;
    const row = state.history.find((r) => r.id === id);
    if (!row) return { ids: [], exit_code: null } satisfies TweaksApplied;
    const original = ALL.find((a) => a.tweak === row.change.tweak && a.subject === row.change.subject);
    const back: TweakRow = {
      ...row,
      id: nextId++,
      applied_ms: Date.now(),
      change: {
        ...row.change,
        undo_of: id,
        results: row.change.results.map((r) => ({ change: { ...r.change, from: r.change.to, to: r.change.from }, error: null })),
      },
      helped: null,
    };
    state.history = [back, ...state.history.map((r) => (r.id === id ? { ...r, undone_ms: Date.now() } : r))];
    if (original) state.advice = state.advice.map((x) => (x.tweak === row.change.tweak && x.subject === row.change.subject ? original : x));
    return { ids: [back.id], exit_code: row.applier === "Elevated" ? 0 : null } satisfies TweaksApplied;
  },
};
