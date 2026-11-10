import { formatDuration } from "../../lib/format";
import { t, tn } from "../../lib/i18n.svelte";
import type {
  Advice,
  DetailLevel,
  RestorePoint,
  SettingChange,
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

/** A setting's name and value as Windows names them. */
function technical(setting: TweakSetting, value: TweakValue): string {
  const label =
    typeof setting === "string"
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
  let shownValue: string;
  if (value === "Absent" || value === "Present") shownValue = k(`raw.${value}`);
  else if ("Dword" in value) shownValue = String(value.Dword);
  else if ("Index" in value) shownValue = String(value.Index);
  else if ("Scheme" in value) shownValue = value.Scheme;
  else if ("On" in value) shownValue = k(`raw.hiberfile.${value.On ? "on" : "off"}`);
  else if ("Bytes" in value) shownValue = value.Bytes.slice(0, 1).map((b) => b.toString(16).padStart(2, "0")).join("");
  else
    shownValue = k("raw.service", {
      start: value.Service.start,
      delayed: value.Service.delayed ? k("raw.delayed") : "",
      state: k(value.Service.running ? "raw.running" : "raw.stopped"),
    });
  return `${label} = ${shownValue}`;
}

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

export function helpedVerdict(before: number, after: number): string {
  return k(after < before ? "helped.fewer" : after > before ? "helped.more" : "helped.same");
}

/** Where a history row stands, in words. */
export function rowState(r: TweakRow): { mark: "Ok" | "Attention" | "Closed"; text: string } {
  const failed = r.change.results.some((x) => x.error !== null && x.error !== ERROR_INTERRUPTED_WRITTEN);
  const written = r.change.results.some(wrote);
  if (r.change.undo_of !== null) return { mark: "Closed", text: k("row.undo") };
  if (r.undone_ms !== null) return { mark: "Closed", text: k("row.undone", { when: at(r.undone_ms) }) };
  if (!written) return { mark: "Attention", text: k("row.refused") };
  if (r.outside) return { mark: "Attention", text: k("row.outside") };
  return { mark: failed ? "Attention" : "Ok", text: k(failed ? "row.partial" : "row.in_place") };
}

export function restorePoint(p: RestorePoint | null): string {
  if (p === null) return "";
  if (p === "Created") return k("point.created");
  if (p === "Off") return k("point.off");
  if (p === "Unconfirmed") return k("point.unconfirmed");
  if ("Reused" in p) return p.Reused.at_ms ? k("point.reused", { when: at(p.Reused.at_ms) }) : k("point.reused.unknown");
  return k("point.failed", { error: p.Failed.error });
}

