import { formatDuration, formatNumber } from "../../lib/format";
import type { StatusKind } from "../../components/StatusMark.svelte";
import { t, tn } from "../../lib/i18n.svelte";
import type {
  Advice,
  DetailLevel,
  Helped,
  RestorePoint,
  SettingChange,
  SettingResult,
  TweakId,
  TweakRow,
  TweakSetting,
  TweakSignal,
  TweakValue,
} from "../../lib/types";
import { formatBytes, formatDayShort } from "../occurrence/format";
import { at } from "../dumps/words";

// The words of the optimizations page, built from typed advice and history at the reader's level.

type Args = Record<string, string | number>;

const k = (key: string, args?: Args) => t(`ui.tweaks.${key}`, args);
const kn = (key: string, count: number, args?: Args) => tn(`ui.tweaks.${key}`, count, args);

export type Group = "stutters" | "disk" | "falls" | "startup" | "space";

export const GROUP: Record<TweakId, Group> = {
  FastStartup: "falls",
  Hibernation: "space",
  PowerPlan: "stutters",
  PcieAspm: "stutters",
  UsbSuspend: "stutters",
  SysMain: "disk",
  WindowsSearch: "disk",
  GameDvr: "stutters",
  GpuScheduling: "stutters",
  Mpo: "stutters",
  StartupApp: "startup",
  AppDumps: "falls",
  CrashDump: "falls",
};

export const ICON: Record<Group, "wave" | "disk" | "power" | "clock"> = {
  stutters: "wave",
  disk: "disk",
  falls: "power",
  startup: "clock",
  space: "disk",
};

export const TONE: Record<Group, string> = {
  stutters: "press",
  disk: "busy",
  falls: "freeze",
  startup: "press",
  space: "busy",
};

/** What the page calls one tweak's state. */
export type Shown = "signal" | "quiet" | "applied" | "set" | "unavailable" | "protected";

/** Windows reported no error, or reported that a write failed and the old value may not be back. */
const ERROR_MAY_HAVE_CHANGED = 0x2000_0001;
/** The helper stopped before it reported, and Windows holds the value it was to write. */
const ERROR_INTERRUPTED_WRITTEN = 0x2000_0002;
/** The helper stopped before it reported, and Windows still holds the value from before. */
const ERROR_INTERRUPTED_UNCHANGED = 0x2000_0003;
/** The helper stopped before it reported, and Windows holds neither value, or it cannot be read. */
const ERROR_INTERRUPTED_UNKNOWN = 0x2000_0004;
/** The helper found a point Windows logged as created missing from the list. */
const ERROR_RESTORE_DROPPED = 0x2000_0005;
const wrote = (x: { error: number | null }) =>
  x.error === null || x.error === ERROR_MAY_HAVE_CHANGED || x.error === ERROR_INTERRUPTED_WRITTEN;

/** The change still in place for this tweak, if Vestigium made one and Windows still holds it. */
export function inPlace(a: Advice, history: TweakRow[]): TweakRow | null {
  return (
    history.find(
      (r) =>
        r.change.tweak === a.tweak &&
        r.change.subject === a.subject &&
        r.undone_ms === null &&
        r.change.undo_of === null &&
        !r.outside &&
        r.change.results.some(wrote),
    ) ?? null
  );
}

export function shown(a: Advice, history: TweakRow[]): Shown {
  if (a.status === "Ready") return a.signals.length ? "signal" : "quiet";
  if (a.status === "AlreadySet") return inPlace(a, history) ? "applied" : "set";
  return a.status === "Unavailable" ? "unavailable" : "protected";
}

export function name(a: { tweak: TweakId; subject: string | null }, level: DetailLevel): string {
  const subject = a.subject ?? "";
  return level === "Technical" ? k(`name.${a.tweak}.tech`, { name: subject }) : k(`name.${a.tweak}`, { name: subject });
}

export const what = (a: Advice, level: DetailLevel) =>
  k(`what.${a.tweak}${level === "Technical" ? ".tech" : ""}`);
export const risk = (a: Advice) => k(`risk.${a.tweak}`);
export const whyNot = (a: Advice) => k(`why_not.${a.tweak}`);

// ---- What a setting holds, in words ----

/** The setting that says the tweak's state; Game DVR is judged by its background recording. */
function primary(tweak: TweakId): number {
  return tweak === "GameDvr" ? 1 : 0;
}

const SCHEMES: Record<string, string> = {
  "381b4222-f694-41f0-9685-ff5bb260df2e": "balanced",
  "a1841308-3541-4fab-bc81-f71556f20b4a": "saver",
  "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c": "high",
};

/** One value of a tweak's main setting in plain words. */
function plain(tweak: TweakId, value: TweakValue): string {
  const on = (yes: boolean) => k(`state.${tweak}.${yes ? "on" : "off"}`);
  const dword = typeof value === "object" && "Dword" in value ? value.Dword : null;
  switch (tweak) {
    case "PowerPlan":
      return k(`scheme.${(typeof value === "object" && "Scheme" in value && SCHEMES[value.Scheme]) || "other"}`);
    case "PcieAspm":
      return k(`aspm.${typeof value === "object" && "Index" in value ? Math.min(value.Index, 2) : 0}`);
    case "CrashDump":
      return k(`crash.${[0, 1, 2, 3, 7].includes(dword ?? -1) ? dword : "other"}`);
    case "Hibernation":
      return on(typeof value === "object" && "On" in value && value.On);
    case "UsbSuspend":
      return on(typeof value === "object" && "Index" in value && value.Index !== 0);
    case "SysMain":
    case "WindowsSearch": {
      const start = typeof value === "object" && "Service" in value ? value.Service.start : 2;
      return start === 4 ? on(false) : start === 3 ? k("state.manual") : on(true);
    }
    case "Mpo":
      return on(dword !== 5);
    case "GpuScheduling":
      return on(dword === 2);
    case "StartupApp":
      return on(!(typeof value === "object" && "Bytes" in value && (value.Bytes[0] ?? 0) % 2 === 1));
    case "AppDumps":
      return on(value === "Present");
    default:
      return on(dword !== null && dword !== 0);
  }
}

/** Windows' names for the power settings the tweaks write, plugged in. */
const POWER_NAMES: Record<string, string> = {
  "ee12f906-d277-404b-b6da-e5fa1a576df5": "PCIe ASPM (AC)",
  "48e6b7a6-50f5-4782-a5d4-53bb8f07e226": "USB selective suspend (AC)",
};

/** A setting's name as Windows names it. */
function settingLabel(setting: TweakSetting): string {
  return typeof setting === "string"
    ? setting
    : "Dword" in setting
      ? setting.Dword.name
      : "Key" in setting
        ? setting.Key.key.split("\\").pop() ?? ""
        : "Service" in setting
          ? setting.Service.name
          : "PowerAc" in setting
            ? (POWER_NAMES[setting.PowerAc.setting] ?? setting.PowerAc.setting)
            : `StartupApproved\\${setting.StartupApproval.list}\\${setting.StartupApproval.name}`;
}

/** A value as Windows shows it. */
function rawValue(value: TweakValue): string {
  if (value === "Absent" || value === "Present") return k(`raw.${value}`);
  if ("Dword" in value) return String(value.Dword);
  if ("Index" in value) return String(value.Index);
  if ("Scheme" in value) return value.Scheme;
  if ("On" in value) return k(`raw.hiberfile.${value.On ? "on" : "off"}`);
  if ("Bytes" in value) return value.Bytes.slice(0, 1).map((b) => b.toString(16).padStart(2, "0")).join("");
  return k("raw.service", {
    start: value.Service.start,
    delayed: value.Service.delayed ? k("raw.delayed") : "",
    state: k(value.Service.running ? "raw.running" : "raw.stopped"),
  });
}

/** A setting's name and value as Windows names them. */
const technical = (setting: TweakSetting, value: TweakValue) => `${settingLabel(setting)} = ${rawValue(value)}`;

export interface Change {
  from: string;
  to: string;
}

/** What the tweak changes, as one line in plain words or one per setting in Technical. */
export function changeOf(a: { tweak: TweakId }, changes: SettingChange[], level: DetailLevel): Change[] {
  if (!changes.length) return [];
  if (level === "Technical") {
    return changes.map((c) => ({ from: technical(c.setting, c.from), to: technical(c.setting, c.to) }));
  }
  const main = changes[Math.min(primary(a.tweak), changes.length - 1)];
  return [{ from: plain(a.tweak, main.from), to: plain(a.tweak, main.to) }];
}

/** What the tweak's settings hold now. */
export function nowOf(a: Advice, level: DetailLevel): string {
  if (!a.current.length) return "";
  if (level === "Technical") return a.current.map((c) => technical(c.setting, c.value)).join(" · ");
  return plain(a.tweak, a.current[Math.min(primary(a.tweak), a.current.length - 1)].value);
}

// ---- Signals ----

export function signal(s: TweakSignal): string {
  if (s === "DumpsOff") return k("signal.dumps_off");
  if ("Finding" in s) return k(`signal.finding.${s.Finding}`);
  if ("UnfinishedShutdowns" in s) return kn("signal.unfinished", s.UnfinishedShutdowns.count);
  if ("SleepFalls" in s) return kn("signal.sleep_falls", s.SleepFalls.count);
  if ("Stutters" in s) return kn("signal.stutters", s.Stutters.count);
  if ("DpcIsrStalls" in s) return kn("signal.dpc", s.DpcIsrStalls.count);
  if ("GpuDriverStalls" in s) return kn("signal.gpu", s.GpuDriverStalls.count);
  if ("DiskBusy" in s) {
    const { count, culprit } = s.DiskBusy;
    return culprit ? kn("signal.disk.culprit", count, { culprit }) : kn("signal.disk", count);
  }
  if ("CpuSaturated" in s) {
    const { count, culprit } = s.CpuSaturated;
    return culprit ? kn("signal.cpu.culprit", count, { culprit }) : kn("signal.cpu", count);
  }
  if ("CaptureDriver" in s) {
    return k("signal.capture_driver", { day: formatDayShort(s.CaptureDriver.start_ms), module: s.CaptureDriver.module });
  }
  if ("CaptureFrames" in s) {
    return k("signal.capture_frames", { day: formatDayShort(s.CaptureFrames.start_ms), app: s.CaptureFrames.app });
  }
  if ("AppCrashes" in s) return kn("signal.app_crashes", s.AppCrashes.count);
  if ("HiberFile" in s) return k("signal.hiberfile", { size: formatBytes(s.HiberFile.bytes) });
  if ("Monitors" in s) return kn("signal.monitors", s.Monitors.count);
  return k("signal.slow_startup", { time: formatDuration(s.SlowStartup.total_ms) });
}

// ---- History ----

const METRIC: Partial<Record<TweakId, string>> = {
  FastStartup: "falls",
  PowerPlan: "stutters",
  PcieAspm: "stutters",
  UsbSuspend: "stutters",
  GameDvr: "stutters",
  Mpo: "stutters",
  SysMain: "busy",
  WindowsSearch: "busy",
  GpuScheduling: "resets",
};

export const metric = (tweak: TweakId) => k(`helped.metric.${METRIC[tweak] ?? "stutters"}`);

const HOUR_MS = 3_600_000;

/** Whole hours the monitor sampled, rounded down so "only 11 h" never reads as the 12 h asked. */
export const monitoredHours = (ms: number) => Math.floor(ms / HOUR_MS);

/** Events per monitored hour; zero when nothing was monitored. */
export const ratePerHour = (events: number, ms: number) => (ms > 0 ? events / (ms / HOUR_MS) : 0);

/** A rate as written next to its bar: "0,11 por hora", "0" or "<0,01" for a rate that rounds to nothing. */
export function rateText(events: number, ms: number): string {
  const rate = ratePerHour(events, ms);
  if (events === 0) return k("helped.rate", { rate: formatNumber(0) });
  return k("helped.rate", { rate: rate < 0.01 ? `<${formatNumber(0.01, 2)}` : formatNumber(rate, 2) });
}

/** What the comparison says, in words. */
export const helpedWord = (h: Helped) => k(`helped.verdict.${h.verdict}`);

/** Whether the comparison lacks data, and so is drawn as an outline. */
export const lacksData = (h: Helped) => h.verdict === "NotEnoughEvents" || h.verdict === "NotEnoughTime";

/** Why the comparison says nothing yet; `null` once it says something. */
export function helpedWhy(h: Helped): string | null {
  const need = { min: h.min_events };
  if (h.verdict === "NotEnoughEvents") return kn("helped.why.events", h.before + h.after, need);
  if (h.verdict !== "NotEnoughTime") return null;
  const least = h.min_hours * HOUR_MS;
  const args = { before: monitoredHours(h.before_ms), after: monitoredHours(h.after_ms), min: h.min_hours };
  if (h.before_ms < least && h.after_ms < least) return k("helped.why.time.both", args);
  return k(h.before_ms < least ? "helped.why.time.before" : "helped.why.time.after", args);
}

/** What the watcher found one setting holding after the helper stopped; `null` for any other result. */
export type Settle = "written" | "unchanged" | "unknown";

export function settleOf(x: SettingResult): Settle | null {
  if (x.error === ERROR_INTERRUPTED_WRITTEN) return "written";
  if (x.error === ERROR_INTERRUPTED_UNCHANGED) return "unchanged";
  return x.error === ERROR_INTERRUPTED_UNKNOWN ? "unknown" : null;
}

/** How a stopped change ended as a whole: every setting alike, some holding the new value, or none known. */
export type Interrupted = "all" | "none" | "unknown" | "part" | "doubt";

/** The outcome of a change whose helper stopped before it reported; `null` for any other change. */
export function interruptedOf(r: TweakRow): Interrupted | null {
  if (r.change.undo_of !== null) return null;
  const found = r.change.results.map(settleOf).filter((x) => x !== null);
  if (!found.length) return null;
  const count = (s: Settle) => found.filter((x) => x === s).length;
  if (count("written") === found.length) return "all";
  if (count("unchanged") === found.length) return "none";
  if (count("unknown") === found.length) return "unknown";
  return count("written") ? "part" : "doubt";
}

/** The newest change of a tweak when its helper stopped and nothing has happened to it since. */
export function openInterrupted(rows: TweakRow[]): TweakRow | null {
  const [newest] = rows;
  return newest && newest.undone_ms === null && !newest.outside && interruptedOf(newest) !== null ? newest : null;
}

/** Whether Undo has a setting to put back: one holds the new value. */
export const undoable = (r: TweakRow) => r.change.results.some((x) => settleOf(x) === "written");

/** Where a history row stands, in words. */
export function rowState(r: TweakRow): { mark: "Ok" | "Attention" | "Closed"; text: string } {
  const failed = r.change.results.some((x) => x.error !== null && x.error !== ERROR_INTERRUPTED_WRITTEN);
  const written = r.change.results.some(wrote);
  const stopped = interruptedOf(r);
  if (r.change.undo_of !== null) return { mark: "Closed", text: k("row.undo") };
  if (r.undone_ms !== null) return { mark: "Closed", text: k("row.undone", { when: at(r.undone_ms) }) };
  if (stopped && !(r.outside && written)) {
    return { mark: stopped === "none" ? "Closed" : "Attention", text: k(`row.interrupted.${stopped}`) };
  }
  if (!written) return { mark: "Attention", text: k("row.refused") };
  if (r.outside) return { mark: "Attention", text: k("row.outside") };
  return { mark: failed ? "Attention" : "Ok", text: k(failed ? "row.partial" : "row.in_place") };
}

/** The sentence at the head of the note on a stopped change. */
export const interruptedLead = () => k("why.lead");

/** What the stopped change left in Windows, and whether Undo is there for it. */
export function interruptedOutcome(kind: Interrupted, canUndo: boolean): string {
  if (kind === "part" || kind === "doubt") return k("why.mixed") + (canUndo ? ` ${k("why.mixed.undo")}` : "");
  return k(`why.${kind}`);
}

/** Settings with a plain name of their own, by tweak. */
const NAMED: Partial<Record<TweakId, number>> = { GameDvr: 3, AppDumps: 3 };

/** One setting of a stopped change, named, with what was found. */
export interface Found {
  name: string;
  settle: Settle;
  word: string;
}

/** Each setting of a stopped change with what Windows held when the watcher looked. */
export function foundOf(a: { tweak: TweakId; subject: string | null }, r: TweakRow, level: DetailLevel): Found[] {
  return r.change.results.flatMap((x, i) => {
    const settle = settleOf(x);
    if (settle === null) return [];
    const plain = i < (NAMED[a.tweak] ?? 0) ? k(`set.name.${a.tweak}.${i}`) : name(a, "Simple");
    const label = level === "Technical" ? `${settingLabel(x.change.setting)} = ${rawValue(x.change.from)} → ${rawValue(x.change.to)}` : plain;
    return [{ name: label, settle, word: k(`set.${settle}`) }];
  });
}

/** The mark that goes with what one setting was found holding. */
export const settleMark = (s: Settle): StatusKind => (s === "written" ? "ok" : s === "unchanged" ? "info" : "attention");

/** The mark for the stopped change as a whole. */
export const interruptedMark = (kind: Interrupted): StatusKind => (kind === "none" ? "info" : "attention");

/** "Interrupted 2 h ago", for a change whose helper stopped. */
export const interruptedWhen = (r: TweakRow) => k("sub.interrupted", { when: at(r.applied_ms) });

/** What the card shows as current for a setting the helper never confirmed. */
export function unconfirmed(setting: TweakSetting, level: DetailLevel): string {
  const word = k("fact.unconfirmed");
  return level === "Technical" ? `${settingLabel(setting)} = ${word.toLocaleLowerCase()}` : word;
}

export function restorePoint(p: RestorePoint | null): string {
  if (p === null) return "";
  if (p === "Created") return k("point.created");
  if (p === "Off") return k("point.off");
  if (p === "Unconfirmed") return k("point.unconfirmed");
  if ("Reused" in p) return p.Reused.at_ms ? k("point.reused", { when: at(p.Reused.at_ms) }) : k("point.reused.unknown");
  if (p.Failed.error === ERROR_RESTORE_DROPPED) return k("point.dropped");
  return k("point.failed", { error: windowsCode(p.Failed.error) });
}

/** A Windows error as a person looks it up: small codes in decimal, HRESULTs in hex. */
export function windowsCode(error: number): string {
  return error > 0xffff ? `0x${error.toString(16).toUpperCase()}` : String(error);
}

/** Why no restore point stands behind a change. */
export type PointCase = "off" | "failed" | "dropped" | "unconfirmed";

/** One step on the way to a restore point, with where it stopped. */
export interface TrailStep {
  mark: StatusKind;
  name: string;
  word: string;
  /** The step where the path broke. */
  here: boolean;
}

export function pointCase(p: RestorePoint): PointCase {
  if (p === "Off") return "off";
  if (typeof p === "object" && "Failed" in p) return p.Failed.error === ERROR_RESTORE_DROPPED ? "dropped" : "failed";
  return "unconfirmed";
}

/** The dialog's title: a dropped point reads as unconfirmed, since Windows logged it as made. */
export function noPointTitle(c: PointCase): string {
  return k(`no_point.title.${c === "dropped" ? "unconfirmed" : c}`);
}

/** Request to Windows, point made, point listed: what the helper knew of each step. */
export function pointTrail(p: RestorePoint): TrailStep[] {
  const c = pointCase(p);
  const step = (key: string) => k(`no_point.step.${key}`);
  const code = typeof p === "object" && "Failed" in p ? windowsCode(p.Failed.error) : "";
  const rows: [StatusKind, string][] = {
    off: [["attention", step("not_done")], ["info", step("na")], ["info", step("unchecked")]],
    failed: [["ok", step("done")], ["critical", `${step("failed")} · ${k("no_point.error", { code })}`], ["info", step("unchecked")]],
    dropped: [["ok", step("done")], ["ok", step("logged")], ["attention", step("missing")]],
    unconfirmed: [["ok", step("done")], ["attention", step("unconfirmed")], ["info", step("unchecked")]],
  }[c] as [StatusKind, string][];
  const broke = { off: 0, failed: 1, dropped: 2, unconfirmed: 1 }[c];
  return ["ask", "made", "listed"].map((name, i) => ({ mark: rows[i][0], name: step(name), word: rows[i][1], here: i === broke }));
}

/** The line shown when a new attempt ends the same way. */
export function againLine(c: PointCase, attempt: number): string {
  return k(`no_point.again.${c === "dropped" ? "unconfirmed" : c}`, { n: attempt });
}

