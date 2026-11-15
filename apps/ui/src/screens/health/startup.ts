import type { IconName } from "../../components/Icon.svelte";
import type { StatusKind } from "../../components/StatusMark.svelte";
import { formatDuration, formatTime } from "../../lib/format";
import { lang, t, tn } from "../../lib/i18n.svelte";
import type { BootDto, StartupRun, TempSize } from "../../lib/types";
import { formatDayShort, sameDay } from "../occurrence/format";

// The phases of a startup, their normal ranges, and the words of the Startup area.

export type PhaseId = "firmware" | "signin" | "profile" | "desktop" | "apps";

export interface Phase {
  id: PhaseId;
  icon: IconName;
  /** The Windows event that marks the phase, shown at the Technical level. */
  source: string;
  /** The longest normal stretch in seconds; for the sign-in screen, where a wait begins. */
  limit: number | null;
}

export const PHASES: readonly Phase[] = [
  { id: "firmware", icon: "chip", source: "Kernel-Boot 30", limit: 25 },
  { id: "signin", icon: "windows", source: "Winlogon 7001", limit: 30 },
  { id: "profile", icon: "person", source: "User Profile Service", limit: 30 },
  { id: "desktop", icon: "app", source: "Shell-Core 62171", limit: 10 },
  { id: "apps", icon: "start", source: "Shell-Core 9707", limit: null },
];

/** The user profile step above this many seconds is an alert; above its limit it is only above normal. */
export const PROFILE_ALERT_S = 60;
/** A Temp folder with this many folders or more is called large. */
export const TEMP_LARGE = 20_000;

const k = (key: string, args?: Record<string, string | number>) => t(`ui.health.startup.${key}`, args);
const kn = (key: string, count: number, args?: Record<string, string | number>) => tn(`ui.health.startup.${key}`, count, args);

export const phaseOf = (id: PhaseId): Phase => PHASES.find((p) => p.id === id)!;

/** Milliseconds from the desktop to the first startup app; `null` when either time is missing. */
export function appsMs(boot: BootDto): number | null {
  if (boot.to_desktop_ms === null || boot.apps.length === 0) return null;
  const first = Math.min(...boot.apps.map((run) => run.start_ms));
  const ms = first - (boot.kernel_start_ms + boot.to_desktop_ms);
  return ms < 0 ? null : ms;
}

/** How long a phase took in this startup, in milliseconds; `null` when it was not measured. */
export function phaseMs(boot: BootDto, id: PhaseId): number | null {
  if (id === "firmware") return boot.firmware_ms;
  if (id === "signin") return boot.to_signin_ms;
  if (id === "profile") return boot.profile_ms;
  if (id === "desktop") return boot.desktop_ms;
  return appsMs(boot);
}

/** One phase across the startups, in seconds, with a gap where it was not measured. */
export function series(boots: BootDto[], id: PhaseId): (number | null)[] {
  return boots.map((boot) => {
    const ms = phaseMs(boot, id);
    return ms === null ? null : ms / 1000;
  });
}

const NICE = [5, 10, 20, 30, 40, 50, 60, 80, 100, 120, 150, 200, 250, 300, 350, 400, 450, 500, 600, 700, 800, 900, 1000, 1200, 1500];

/** The round number at or above `value`, for the top of a scale. */
export function niceCeiling(value: number): number {
  return NICE.find((n) => n >= value) ?? Math.ceil(value / 500) * 500;
}

/** The top of a phase's scale: round, never under its limit nor any startup shown. */
export function ceilingOf(boots: BootDto[], id: PhaseId): number {
  const highest = Math.max(phaseOf(id).limit ?? 0, ...series(boots, id).map((v) => v ?? 0));
  return niceCeiling(highest);
}

export interface Judgement {
  kind: StatusKind;
  word: string;
}

/** How a phase of one startup stands: a wait or an excess is information, only the profile step can be an alert. */
export function judge(boot: BootDto, id: PhaseId): Judgement {
  const ms = phaseMs(boot, id);
  if (ms === null) return { kind: "info", word: k("word.unmeasured") };
  const s = ms / 1000;
  if (id === "signin" && boot.probable_wait) return { kind: "info", word: k("word.wait") };
  if (id === "profile" && s > PROFILE_ALERT_S) return { kind: "attention", word: t("ui.health.mark.Attention") };
  const limit = phaseOf(id).limit;
  if (id !== "signin" && limit !== null && s > limit) return { kind: "info", word: k("word.above") };
  return { kind: "ok", word: t("ui.health.mark.Ok") };
}

/** A duration for a phase or a total: seconds up to two minutes, then "2 min 32 s". */
export function formatSeconds(ms: number): string {
  const s = ms / 1000;
  if (s >= 3600) return formatDuration(ms);
  const total = Math.round(s);
  if (total >= 120) {
    const minutes = Math.floor(total / 60);
    return total % 60 ? `${minutes} min ${total % 60} s` : `${minutes} min`;
  }
  const digits = s >= 100 ? 0 : 1;
  return `${new Intl.NumberFormat(lang(), { maximumFractionDigits: digits }).format(s)} s`;
}

/** A phase's value, or "not measured" where it has none; never zero. */
export function valueText(ms: number | null): string {
  return ms === null ? k("value.unmeasured") : formatSeconds(ms);
}

/** What counts as normal for a phase, written under its value. */
export function rangeText(id: PhaseId): string {
  return k(`range.${id}`, { limit: phaseOf(id).limit ?? 0 });
}

/** "today, 14:36" or "Nov 13, 11:42". */
export function stamp(ms: number): string {
  return sameDay(ms, Date.now()) ? t("ui.health.today_at", { time: formatTime(ms) }) : `${formatDayShort(ms)}, ${formatTime(ms)}`;
}

export interface Part {
  text: string;
  bold: boolean;
}

/** The sentence under the title: how long the last startup took, and when. */
export function ledeParts(boots: BootDto[]): Part[] {
  const last = boots[boots.length - 1];
  if (!last) return [{ text: k("lede.none"), bold: true }, { text: ` ${k("lede.none.rest")}`, bold: false }];
  const when = stamp(last.kernel_start_ms);
  const wait = last.probable_wait ? k("lede.wait") : "";
  const total = last.from_power_ms ?? last.to_desktop_ms;
  if (total === null) return [{ text: `${k("lede.unmeasured", { when })}${wait}.`, bold: false }];
  const rest = k(last.from_power_ms !== null ? "lede.power" : "lede.kernel", { when });
  return [
    { text: `${k("lede.lead")} `, bold: false },
    { text: formatSeconds(total), bold: true },
    { text: ` ${rest}${wait}.`, bold: false },
  ];
}

/** A startup's total, counted the way the sentence at the top counts it. */
export function totalText(boot: BootDto): string {
  if (boot.from_power_ms !== null) return k("boots.total.power", { time: formatSeconds(boot.from_power_ms) });
  if (boot.to_desktop_ms !== null) return k("boots.total.kernel", { time: formatSeconds(boot.to_desktop_ms) });
  return k("boots.total.none");
}

/** The line under a startup in the list: its profile step, and a wait on the sign-in screen. */
export function bootSub(boot: BootDto): string {
  const profile = boot.profile_ms === null ? k("boots.profile_none") : k("boots.profile", { time: formatSeconds(boot.profile_ms) });
  return boot.probable_wait ? `${profile} · ${k("boots.wait")}` : profile;
}

/** The part of the sentence about the TPM: how long the profile waited for it, if known. */
export function tpmText(boot: BootDto): string | null {
  if (boot.tpm_wait_ms === null) return null;
  return boot.tpm_wait_ms > 0 ? k("body.tpm_waited", { time: formatSeconds(boot.tpm_wait_ms) }) : k("body.tpm_ready");
}

/** What a phase's row says when opened, apart from the profile step's alert. */
export function causeText(boot: BootDto, id: PhaseId): string {
  const judgement = judge(boot, id);
  if (id === "apps") return k("body.apps");
  if (phaseMs(boot, id) === null) return k("body.unmeasured");
  if (id === "signin") return k("body.signin", { limit: phaseOf("signin").limit ?? 0 });
  return k(judgement.kind === "ok" ? "body.ok" : "body.above");
}

/** The failed drivers across the startups, most often first, each with how many startups it failed in. */
export function failures(boots: BootDto[]): { name: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const boot of boots) for (const name of new Set(boot.failed_drivers)) counts.set(name, (counts.get(name) ?? 0) + 1);
  return [...counts].map(([name, count]) => ({ name, count })).sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
}

/** The line under a failed driver. */
export function failureSub(count: number, total: number): string {
  return kn("fail.sub", total, { count, total });
}

/** "Opened at 81 s after Windows started · took 2,8 s". */
export function runText(run: StartupRun, boot: BootDto): string {
  const opened = k("run.opened", { at: formatSeconds(run.start_ms - boot.kernel_start_ms) });
  return run.end_ms === null ? opened : `${opened} · ${k("run.took", { took: formatSeconds(run.end_ms - run.start_ms) })}`;
}

/** The line under the Temp folder: how many folders and when they were counted. */
export function tempSub(temp: TempSize | null): string {
  if (temp === null) return k("temp.unmeasured");
  const counted = sameDay(temp.ts_ms, Date.now())
    ? k("temp.counted.today", { time: formatTime(temp.ts_ms) })
    : k("temp.counted.day", { date: stamp(temp.ts_ms) });
  return kn(temp.capped ? "temp.sub.capped" : "temp.sub", temp.folders, { counted });
}

export const tempIsLarge = (temp: TempSize | null): boolean => temp !== null && temp.folders >= TEMP_LARGE;

/** "Nov 13, 11:42": the day and the time of a startup, never "today", so a title reads in either language. */
export function startupWhen(ms: number): string {
  return `${formatDayShort(ms)}, ${formatTime(ms)}`;
}

/** The kind of startup the system recorded, as the suffix of its text key; `null` when it did not say. */
export function bootKind(boot: BootDto): "full" | "fast" | "resume" | null {
  if (boot.boot_type === 0) return "full";
  if (boot.boot_type === 1) return "fast";
  if (boot.boot_type === 2) return "resume";
  return null;
}

/** The startups before and after the one starting at `ms`, in the order the list holds them (oldest first). */
export function neighbours(boots: BootDto[], ms: number): { boot: BootDto | null; prev: BootDto | null; next: BootDto | null } {
  const at = boots.findIndex((b) => b.kernel_start_ms === ms);
  if (at < 0) return { boot: null, prev: null, next: null };
  return { boot: boots[at], prev: boots[at - 1] ?? null, next: boots[at + 1] ?? null };
}

/** How long the profile took once the TPM was ready; `null` when no wait was recorded. */
export function profileAfterTpmMs(boot: BootDto): number | null {
  if (boot.tpm_wait_ms === null || boot.tpm_wait_ms <= 0 || boot.profile_ms === null) return null;
  return Math.max(0, boot.profile_ms - boot.tpm_wait_ms);
}

/** A phase placed on the seconds axis; both ends are `null` when its place or length is not known. */
export interface Segment {
  id: PhaseId;
  from: number | null;
  to: number | null;
}

export interface Waterfall {
  segments: Segment[];
  /** Firmware time is missing, so the axis counts from the start of Windows. */
  fromWindows: boolean;
  /** Where the TPM became ready, on the same axis; `null` when the profile did not wait for it. */
  tpmAt: number | null;
  /** The last second any phase reaches. */
  end: number;
}

/** The phases of one startup as segments on an axis of seconds from the power button, or from Windows when firmware is missing. */
export function waterfall(boot: BootDto): Waterfall {
  const fw = boot.firmware_ms ?? 0;
  const place = (id: PhaseId, from: number | null, to: number | null): Segment => ({
    id,
    from: from === null || to === null ? null : from / 1000,
    to: from === null || to === null ? null : to / 1000,
  });
  const after = (from: number | null, ms: number | null) => (from === null || ms === null ? null : from + ms);

  const signinEnd = after(fw, boot.to_signin_ms);
  const profileEnd = after(signinEnd, boot.profile_ms);
  const desktopEnd = boot.to_desktop_ms === null ? null : fw + boot.to_desktop_ms;
  const segments = [
    place("firmware", boot.firmware_ms === null ? null : 0, boot.firmware_ms),
    place("signin", fw, signinEnd),
    place("profile", signinEnd, profileEnd),
    place("desktop", profileEnd, boot.desktop_ms === null ? null : desktopEnd),
    place("apps", desktopEnd, after(desktopEnd, appsMs(boot))),
  ];
  const tpm = boot.tpm_wait_ms !== null && boot.tpm_wait_ms > 0 ? after(signinEnd, boot.tpm_wait_ms) : null;
  return {
    segments,
    fromWindows: boot.firmware_ms === null,
    tpmAt: tpm === null ? null : tpm / 1000,
    end: Math.max(0, ...segments.map((s) => s.to ?? 0)),
  };
}

/** The top of the waterfall's axis: round, with a little room after the last phase. */
export const waterfallCeiling = (w: Waterfall): number => niceCeiling(w.end * 1.03);
