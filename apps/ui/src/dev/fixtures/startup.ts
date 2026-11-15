// Startup page data. URL: ?startup=slow|fast|wait|none (default slow)
// slow: the latest boot spends minutes in the profile step; fast: Temp was emptied and it takes seconds;
// wait: the latest boot sat 708 s at the sign-in screen; none: nothing read yet.
// Every name and path here is invented; times follow a real PC's boots, moved to an invented base time.
import type { BootDto, StartupAppDto, StartupAreaDto, StartupRun } from "../../lib/types";
import type { Handler } from "../mock";

export type Case = "slow" | "fast" | "wait" | "none";

const query = new URLSearchParams(location.search);
const MINUTE = 60_000;
const now = Date.now();

/** One boot as measured: seconds after the kernel started, except `fw` (before it); null where missing. */
interface Row {
  at: string;
  fw: number | null;
  signin: number;
  profile: number;
  tpmReady: number | null;
  desktop: number | null;
  apps: number | null;
  type?: number;
  drivers?: string[];
}

const ROWS: Row[] = [
  { at: "2026-09-20 09:53", fw: 14.7, signin: 18, profile: 38, tpmReady: 52, desktop: null, apps: null },
  { at: "2026-09-21 12:00", fw: 15.5, signin: 14, profile: 30, tpmReady: 46, desktop: null, apps: null },
  { at: "2026-09-23 11:53", fw: 31.5, signin: 16, profile: 35, tpmReady: 49, desktop: null, apps: null },
  { at: "2026-09-24 11:44", fw: 19.7, signin: 18, profile: 69, tpmReady: 74, desktop: null, apps: null },
  { at: "2026-09-25 11:43", fw: 15.6, signin: 13, profile: 28, tpmReady: 38, desktop: null, apps: null },
  { at: "2026-09-26 12:31", fw: 15.0, signin: 20, profile: 26, tpmReady: 43, desktop: null, apps: null },
  { at: "2026-10-02 10:31", fw: 31.4, signin: 50, profile: 44, tpmReady: 91, desktop: null, apps: null },
  { at: "2026-10-03 09:47", fw: 19.9, signin: 15, profile: 29, tpmReady: 40, desktop: null, apps: null },
  { at: "2026-10-07 10:41", fw: 14.5, signin: 14, profile: 49, tpmReady: 60, desktop: null, apps: null },
  { at: "2026-10-10 10:00", fw: 15.5, signin: 39, profile: 62, tpmReady: 97, desktop: null, apps: null },
  { at: "2026-10-15 10:32", fw: 14.5, signin: 14, profile: 39, tpmReady: 48, desktop: null, apps: null },
  { at: "2026-10-16 11:49", fw: 15.1, signin: 16, profile: 58, tpmReady: 68, desktop: null, apps: null },
  { at: "2026-10-24 12:42", fw: 14.4, signin: 17, profile: 32, tpmReady: 46, desktop: null, apps: null },
  { at: "2026-10-28 11:50", fw: 15.4, signin: 19, profile: 151, tpmReady: 57, desktop: null, apps: null },
  { at: "2026-10-31 12:43", fw: 15.5, signin: 708, profile: 134, tpmReady: null, desktop: 843, apps: null },
  { at: "2026-11-05 11:42", fw: 15.0, signin: 554, profile: 153, tpmReady: null, desktop: 709, apps: null },
  { at: "2026-11-07 18:40", fw: 21.1, signin: 41, profile: 159, tpmReady: 92, desktop: 201, apps: null },
  { at: "2026-11-08 09:37", fw: 19.7, signin: 17, profile: 183, tpmReady: 58, desktop: 201, apps: null, drivers: ["dam", "exampledrv"] },
  { at: "2026-11-08 09:45", fw: 13.8, signin: 9, profile: 110, tpmReady: 33, desktop: 121, apps: null, type: 1 },
  { at: "2026-11-09 11:43", fw: 15.2, signin: 28, profile: 361, tpmReady: 79, desktop: 390, apps: null, drivers: ["dam"] },
  { at: "2026-11-10 11:53", fw: 14.9, signin: 15, profile: 208, tpmReady: 83, desktop: 224, apps: null, drivers: ["dam"] },
  { at: "2026-11-13 11:42", fw: 14.9, signin: 21, profile: 123, tpmReady: 51, desktop: 145, apps: null, drivers: ["dam"] },
  { at: "2026-11-15 11:15", fw: 15.2, signin: 19, profile: 152, tpmReady: 56, desktop: 173, apps: null, drivers: ["dam"] },
  { at: "2026-11-15 13:11", fw: null, signin: 16.6, profile: 144, tpmReady: 38.9, desktop: null, apps: null },
  { at: "2026-11-15 13:40", fw: 13.6, signin: 20.3, profile: 43.1, tpmReady: 57.3, desktop: 70.6, apps: 81.0 },
  { at: "2026-11-15 13:57", fw: 13.6, signin: 20.6, profile: 50.6, tpmReady: 66.4, desktop: 79.4, apps: 89.9 },
  { at: "2026-11-15 14:08", fw: 17.9, signin: 20.5, profile: 52.3, tpmReady: 63.6, desktop: 82.2, apps: 92.7 },
  { at: "2026-11-15 14:20", fw: 17.7, signin: 18.4, profile: 31.0, tpmReady: null, desktop: 56.3, apps: 67.4 },
  { at: "2026-11-15 14:36", fw: 13.5, signin: 20.7, profile: 8.2, tpmReady: null, desktop: 36.3, apps: 46.9 },
];
/** How many rows each case shows: the rows up to the one the case ends on. */
const LAST: Record<Exclude<Case, "none">, string> = {
  slow: "2026-11-15 11:15",
  fast: "2026-11-15 14:36",
  wait: "2026-10-31 12:43",
};

const ms = (seconds: number) => Math.round(seconds * 1000);
const stamp = (at: string) => Date.parse(`${at.replace(" ", "T")}:00Z`);

/** The program file names the window gets: the Rust side drops the folder and the arguments. */
const APP_COMMANDS = ["vestigium.exe", "capture.exe"];

function runs(row: Row, start: number): StartupRun[] {
  if (row.apps === null) return [];
  const first = start + ms(row.apps);
  return [
    { command: APP_COMMANDS[0], start_ms: first, end_ms: first + 2_800 },
    { command: APP_COMMANDS[1], start_ms: first + 3_300, end_ms: first + 4_000 },
  ];
}

/** The boot the way the Rust side works its phases out, `start` being the kernel's start in epoch ms. */
function boot(row: Row, start: number): BootDto {
  const profileEnd = row.signin + row.profile;
  const span = (seconds: number | null) => (seconds === null || seconds < 0 ? null : ms(seconds));
  return {
    kernel_start_ms: start,
    boot_type: row.type ?? 0,
    firmware_ms: row.fw === null ? null : ms(row.fw),
    to_signin_ms: ms(row.signin),
    probable_wait: row.signin > 30,
    profile_ms: ms(row.profile),
    tpm_wait_ms: row.tpmReady === null ? null : span(Math.min(row.tpmReady, profileEnd) - row.signin) ?? 0,
    desktop_ms: row.desktop === null ? null : span(row.desktop - profileEnd),
    to_desktop_ms: row.desktop === null ? null : ms(row.desktop),
    from_power_ms: row.fw === null || row.desktop === null ? null : ms(row.fw + row.desktop),
    apps: runs(row, start),
    logger_full: false,
    failed_drivers: row.drivers ?? [],
  };
}

/** Boots up to the one the case ends on, the last starting 20 minutes ago; the gaps between them are the real ones. */
function boots(which: Exclude<Case, "none">): BootDto[] {
  const rows = ROWS.slice(0, ROWS.findIndex((r) => r.at === LAST[which]) + 1);
  const newest = stamp(rows[rows.length - 1].at);
  return rows.map((row) => boot(row, now - 20 * MINUTE - (newest - stamp(row.at))));
}

const TEMP = {
  slow: { folders: 213_879, files: 80_958 },
  fast: { folders: 3_558, files: 3_547 },
  wait: { folders: 137_412, files: 40_210 },
};

const startupApps: StartupAppDto[] = [
  { name: "Vestigium", key: "StartupApp:Vestigium", enabled: true, protected: false },
  { name: "ChatClient", key: "StartupApp:ChatClient", enabled: true, protected: false },
  { name: "NotesSync", key: "StartupApp:NotesSync", enabled: true, protected: false },
  { name: "Example Capture", key: "StartupApp:Example Capture", enabled: false, protected: false },
  { name: "SecurityTray", key: "StartupApp:SecurityTray", enabled: true, protected: true },
];

/** The page's data for one case; the boots are dated relative to now. */
export function startupCase(which: Case): StartupAreaDto {
  if (which === "none") return { boots: [], temp: null, startup_apps: startupApps };
  return {
    boots: boots(which),
    temp: { ts_ms: now - 25 * MINUTE, ...TEMP[which], capped: false },
    startup_apps: startupApps,
  };
}

function data(): StartupAreaDto {
  const wanted = query.get("startup");
  return startupCase(wanted === "fast" || wanted === "wait" || wanted === "none" ? wanted : "slow");
}

export const handlers: Record<string, Handler> = {
  startup_area: () => data(),
  open_storage_sense: () => null,
};
