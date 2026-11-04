// Health page data. URL: ?health=none|attention|critical (default critical) &scan-never=1 &scan-ms=<wait> &scan-fail=1
import type { FindingDto, HealthDto, ScanFinished } from "../../lib/types";
import type { Handler } from "../mock";

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

function data(): HealthDto {
  const level = query.get("health") ?? "critical";
  const open =
    level === "none" ? [] : level === "attention" ? [unfinishedShutdowns, lowSpace] : [failingDisk, unfinishedShutdowns, lowSpace];
  return {
    open,
    closed: [slowBoot],
    last_scan_ms: lastScan,
    boot: {
      boots: [{ ts_ms: now - 12 * DAY, boot_ms: 108_457, main_path_ms: 68_757, post_boot_ms: 39_700 }],
      delays: [],
    },
  };
}

export const handlers: Record<string, Handler> = {
  health: () => data(),
  start_scan: async () => {
    await new Promise((resolve) => setTimeout(resolve, Number(query.get("scan-ms") ?? 1500)));
    if (query.get("scan-fail") === "1") return { scans: [], exit_code: 1 } satisfies ScanFinished;
    lastScan = Date.now();
    return { scans: [1], exit_code: 0 } satisfies ScanFinished;
  },
};
