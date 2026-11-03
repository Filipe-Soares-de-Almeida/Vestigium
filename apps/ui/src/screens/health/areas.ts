import type { IconName } from "../../components/Icon.svelte";
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

export interface Check {
  kind: FindingKind;
  mark: "Ok" | "Unknown";
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
}

export function areaOf(kind: FindingKind): Area {
  return AREAS.find((a) => a.kinds.includes(kind)) ?? AREAS[0];
}

export function findArea(id: string | null): Area | null {
  return AREAS.find((a) => a.id === id) ?? null;
}

export function areaState(area: Area, dto: HealthDto): AreaState {
  const open = dto.open
    .filter((f) => area.kinds.includes(f.kind))
    .sort((a, b) => rank(b.concern) - rank(a.concern) || a.opened_ms - b.opened_ms);
  const closed = dto.closed.filter((f) => area.kinds.includes(f.kind));
  const checks: Check[] = area.kinds
    .filter((kind) => !open.some((f) => f.kind === kind) && !FROM_TESTS.includes(kind))
    .map((kind) => ({
      kind,
      mark: NEEDS_SCAN.includes(kind) && dto.last_scan_ms === null ? "Unknown" : "Ok",
    }));
  const mark: Mark = open[0]?.concern ?? (checks.some((c) => c.mark === "Unknown") ? "Unknown" : "Ok");
  return { area, open, closed, checks, mark };
}

function rank(concern: Concern): number {
  return concern === "Critical" ? 2 : 1;
}
