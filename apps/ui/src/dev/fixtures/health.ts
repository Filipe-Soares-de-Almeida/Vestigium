// Health page data. URL: ?health=none|attention|critical (default critical) &scan-never=1 &scan-ms=<wait> &scan-fail=1
import type { FindingDto, HealthDto, ScanFinished } from "../../lib/types";
import type { Handler } from "../mock";
import { holdRead } from "./helper";

const query = new URLSearchParams(location.search);
const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const now = Date.now();
let lastScan: number | null = query.get("scan-never") === "1" ? null : now - 3 * DAY;

const failingDisk: FindingDto = {
  id: 1,
  opened_ms: now - 4 * DAY,
  updated_ms: now - HOUR,
  closed_ms: null,
  kind: "DiskSmart",
  subject: { Disk: "1 D:" },
  concern: "Critical",
  value: { Count: 24 },
  first_value: { Count: 16 },
  reference: { AtMost: { Count: 0 } },
  evidence: [
    "PredictFailure",
    { Attribute: { id: 5, current: 61, worst: 61, raw: 16 } },
    { Attribute: { id: 197, current: 100, worst: 100, raw: 8 } },
  ],
  next_steps: ["BackUpNow", "ReplaceDisk"],
};

const unfinishedShutdowns: FindingDto = {
  id: 2,
  opened_ms: now - 6 * DAY,
  updated_ms: now - 10 * HOUR,
  closed_ms: null,
  kind: "ShutdownUnfinished",
  subject: "Pc",
  concern: "Attention",
  value: { Count: 7 },
  first_value: { Count: 3 },
  reference: { AtMost: { Count: 0 } },
  evidence: [1, 4, 8, 13, 17].map((days) => ({
    Event: {
      ts_ms: now - days * DAY,
      provider: "Microsoft-Windows-Kernel-Power",
      id: 41,
      meaning: "PowerLoss" as const,
    },
  })),
  next_steps: ["WaitForShutdown"],
};

const lowSpace: FindingDto = {
  id: 3,
  opened_ms: now - 3 * DAY,
  updated_ms: now - HOUR,
  closed_ms: null,
  kind: "LowSpace",
  subject: { Volume: "C:\\" },
  concern: "Attention",
  value: { Percent: 7.4 },
  first_value: { Percent: 9.1 },
  reference: { AtLeast: { Percent: 10 } },
  evidence: [{ Space: { free_bytes: 66 * 2 ** 30, total_bytes: 893 * 2 ** 30 } }],
  next_steps: ["FreeDiskSpace"],
};

const slowBoot: FindingDto = {
  id: 4,
  opened_ms: now - 12 * DAY,
  updated_ms: now - 12 * DAY,
  closed_ms: now - 2 * DAY,
  kind: "SlowBoot",
  subject: "Pc",
  concern: "Attention",
  value: { Seconds: 108.5 },
  first_value: null,
  reference: { AtMost: { Seconds: 60 } },
  evidence: [
    { Boot: { ts_ms: now - 12 * DAY, boot_ms: 108_457, main_path_ms: 68_757, post_boot_ms: 39_700 } },
    {
      Delay: {
        ts_ms: now - 12 * DAY,
        kind: "Phase",
        name: "PreShellInit",
        total_ms: 39_496,
        degradation_ms: 21_117,
        security: false,
      },
    },
  ],
  next_steps: [],
};

/** A slow profile step, open while `?startup=slow` or `?startup=wait` names a slow latest startup. */
function slowProfile(which: "slow" | "wait"): FindingDto {
  const slow = which === "slow";
  const boot = now - 20 * 60_000;
  return {
    id: 5,
    opened_ms: now - 18 * DAY,
    updated_ms: boot,
    closed_ms: null,
    kind: "SlowBoot",
    subject: "Pc",
    concern: "Attention",
    value: { Seconds: slow ? 152 : 134 },
    first_value: { Seconds: 151 },
    reference: { AtMost: { Seconds: 60 } },
    evidence: [
      {
        BootPhases: {
          ts_ms: boot,
          firmware_ms: slow ? 15_200 : 15_500,
          to_signin_ms: slow ? 19_000 : 708_000,
          profile_ms: slow ? 152_000 : 134_000,
          tpm_wait_ms: slow ? 37_000 : null,
          to_desktop_ms: slow ? 173_000 : 843_000,
          from_power_ms: slow ? 188_200 : 858_500,
        },
      },
      { TempFolder: { ts_ms: now - 25 * 60_000, folders: slow ? 213_879 : 137_412, files: slow ? 80_958 : 40_210, capped: false } },
    ],
    next_steps: ["CleanTemp"],
  };
}

function data(): HealthDto {
  const level = query.get("health") ?? "critical";
  const startup = query.get("startup");
  const base =
    level === "none" ? [] : level === "attention" ? [unfinishedShutdowns, lowSpace] : [failingDisk, unfinishedShutdowns, lowSpace];
  const open = startup === "slow" || startup === "wait" ? [...base, slowProfile(startup)] : base;
  return {
    open,
    closed: [slowBoot],
    last_scan_ms: lastScan,
    boot: {
      boots: [{ ts_ms: now - 12 * DAY, boot_ms: 108_457, main_path_ms: 68_757, post_boot_ms: 39_700 }],
      delays: [],
      skipped: 0,
    },
  };
}

export const handlers: Record<string, Handler> = {
  health: () => data(),
  start_scan: async () => {
    await holdRead("scan", Number(query.get("scan-ms") ?? 1500), query.get("scan-hang") === "1");
    if (query.get("scan-fail") === "1") return { scans: [], exit_code: 1 } satisfies ScanFinished;
    lastScan = Date.now();
    return { scans: [1], exit_code: 0 } satisfies ScanFinished;
  },
};
