import type { IconName } from "../../components/Icon.svelte";
import type { StatusKind } from "../../components/StatusMark.svelte";
import type { PassState } from "../../lib/monitor";
import type { Concern, FindingDto, FindingKind, HealthDto } from "../../lib/types";

// The PC by area, as Windows Settings groups it: each finding kind belongs to one area.

export type AreaId = "disks" | "falls" | "memory" | "startup" | "hardware" | "protection";

export interface Area {
  id: AreaId;
  icon: IconName;
  kinds: FindingKind[];
}

export const AREAS: Area[] = [
  { id: "disks", icon: "disk", kinds: ["DiskSmart", "StorageErrors", "DiskUnreadable", "VolumeErrors", "LowSpace", "WindowsOnHdd"] },
  { id: "falls", icon: "power", kinds: ["DirtyRestarts", "ShutdownUnfinished", "FailuresRising", "NoCrashDump"] },
  { id: "memory", icon: "memory", kinds: ["MemoryOften", "PageFile", "MemoryTestErrors"] },
  { id: "startup", icon: "clock", kinds: ["SlowBoot"] },
  { id: "hardware", icon: "chip", kinds: ["HardwareErrors", "DisplayResets", "Throttling"] },
  { id: "protection", icon: "shield", kinds: ["TwoAntivirus"] },
];

/** What the watcher can read only through a full scan. */
const NEEDS_SCAN: FindingKind[] = ["SlowBoot"];
/** What only a test the person runs can find; with no finding there is nothing to claim. */
const FROM_TESTS: FindingKind[] = ["MemoryTestErrors", "DiskUnreadable", "VolumeErrors"];

/** How an area or a check stands: a finding's concern, nothing to say, or not read yet. */
export type Mark = Concern | "Ok" | "Unknown";

/** Why a check is unknown: it needs a full scan, the last pass could not read it, or no pass is current. */
export type Why = "scan" | "unread" | "stale" | "silent";

export interface Check {
  kind: FindingKind;
  mark: "Ok" | "Unknown";
  why: Why | null;
}

export interface AreaState {
  area: Area;
  /** Open findings, critical first. */
  open: FindingDto[];
  /** Findings that closed in the last 30 days. */
  closed: FindingDto[];
  /** The kinds with no open finding. */
  checks: Check[];
  mark: Mark;
  /** What makes the area unknown, when it is. */
  why: Why | null;
}

/** The mark of an area or a check; an unknown one that was not read or is out of date says so. */
export function statusKindOf(mark: Mark, why: Why | null = null): StatusKind {
  if (mark === "Critical") return "critical";
  if (mark === "Attention") return "attention";
  if (mark === "Ok") return "ok";
  if (why === "silent") return "silent";
  return why === "stale" || why === "unread" ? "unverified" : "info";
}

export function areaOf(kind: FindingKind): Area {
  return AREAS.find((a) => a.kinds.includes(kind)) ?? AREAS[0];
}

export function findArea(id: string | null): Area | null {
  return AREAS.find((a) => a.id === id) ?? null;
}

/** Why `kind` cannot be called fine: a scan that never ran, a pass that did not read it, or no current pass. */
function whyUnknown(kind: FindingKind, dto: HealthDto, pass: PassState | null): Why | null {
  if (NEEDS_SCAN.includes(kind)) return dto.last_scan_ms === null ? "scan" : null;
  if (pass?.silent) return "silent";
  if (pass?.stale) return "stale";
  return pass?.unread.includes(kind) ? "unread" : null;
}

/** An area's state; `pass` is how the hourly check stands, or `null` when nothing is known of it. */
export function areaState(area: Area, dto: HealthDto, pass: PassState | null = null): AreaState {
  const open = dto.open
    .filter((f) => area.kinds.includes(f.kind))
    .sort((a, b) => rank(b.concern) - rank(a.concern) || a.opened_ms - b.opened_ms);
  const closed = dto.closed.filter((f) => area.kinds.includes(f.kind));
  const checks: Check[] = area.kinds
    .filter((kind) => !open.some((f) => f.kind === kind) && !FROM_TESTS.includes(kind))
    .map((kind): Check => {
      const why = whyUnknown(kind, dto, pass);
      return { kind, mark: why ? "Unknown" : "Ok", why };
    });
  const unknown = checks.find((c) => c.mark === "Unknown");
  const mark: Mark = open[0]?.concern ?? (unknown ? "Unknown" : "Ok");
  const why = open.length ? null : (checks.find((c) => c.why !== null && c.why !== "scan") ?? unknown)?.why ?? null;
  return { area, open, closed, checks, mark, why };
}

function rank(concern: Concern): number {
  return concern === "Critical" ? 2 : 1;
}
