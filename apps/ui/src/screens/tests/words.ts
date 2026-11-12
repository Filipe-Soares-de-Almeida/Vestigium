import type { MarkKind } from "../../components/ConcernMark.svelte";
import { formatNumber, formatPercent } from "../../lib/format";
import { t, tn } from "../../lib/i18n.svelte";
import type {
  BenchMode,
  DetailLevel,
  DiskDetailDto,
  DiskDto,
  Media,
  MemoryDiagnostic,
  MemoryQuick,
  NvmeHealth,
  Region,
  RunningTest,
  SmartAttribute,
  Subject,
  SurfaceScan,
  TestKind,
  TestRunDto,
  TestsDto,
  VolumeCheck,
  Benchmark,
} from "../../lib/types";
import { formatBytes } from "../occurrence/format";

// The words of the Tests page: which device a test belongs to, how each device stands, and results.

/** A device of the page: the memory, or a physical disk by its number. */
export type Device = "memory" | number;

export const deviceKey = (d: Device) => (d === "memory" ? "memory" : String(d));

/** SMART attributes worth naming; the ones in WATCHED are the ones that warn. */
export const WATCHED = [5, 187, 188, 197, 198];
const NAMED = [1, 3, 4, 5, 7, 9, 10, 12, 187, 188, 190, 194, 196, 197, 198, 199, 241, 242];
export const attrName = (id: number) => (NAMED.includes(id) ? t(`ui.tests.attr.${id}`) : t("ui.tests.attr.vendor"));

/** Whether a SMART attribute warns: a watched count above zero on a hard drive, 188 by its low 16 bits. */
export const smartWarns = (a: SmartAttribute, media: Media) =>
  media === "Hdd" && WATCHED.includes(a.id) && (a.id === 188 ? a.raw % 0x10000 : a.raw) > 0;

type Warn = "attention" | "critical" | null;

/** The level of each NVMe health row, as the Health findings rate the same values. */
export const nvmeMarks = (h: NvmeHealth): { used: Warn; spare: Warn; media: Warn } => ({
  used: h.percentage_used >= 90 ? "attention" : null,
  spare: h.available_spare_pct < h.spare_threshold_pct ? "critical" : null,
  media: h.media_errors > 0 ? "attention" : null,
});

/** Bands of the surface scan, as the map and its legend name them. */
export const BANDS = ["0", "1", "2", "3", "4", "5"] as const;

const isDisk = (s: Subject, disk: DiskDto) =>
  typeof s === "object" && "Disk" in s && (s.Disk === disk.name || s.Disk === String(disk.number));
const isVolume = (s: Subject, root: string) => typeof s === "object" && "Volume" in s && s.Volume === root;

export function lastRun(dto: TestsDto, kind: TestKind, match: (s: Subject) => boolean = () => true): TestRunDto | null {
  const runs = dto.runs.filter((r) => kindOf(r) === kind && match(r.subject));
  return runs.length ? runs[runs.length - 1] : null;
}

export function kindOf(r: { data: TestRunDto["data"] }): TestKind {
  if ("MemoryQuick" in r.data) return "MemoryQuick";
  if ("DiskSurface" in r.data) return "DiskSurface";
  if ("VolumeCheck" in r.data) return "VolumeCheck";
  return "DiskBenchmark";
}

export const quickOf = (r: TestRunDto) => ("MemoryQuick" in r.data ? r.data.MemoryQuick : null) as MemoryQuick | null;
export const surfaceOf = (r: TestRunDto) => ("DiskSurface" in r.data ? r.data.DiskSurface : null) as SurfaceScan | null;
export const checkOf = (r: TestRunDto) => ("VolumeCheck" in r.data ? r.data.VolumeCheck : null) as VolumeCheck | null;
export const benchOf = (r: TestRunDto) => ("DiskBenchmark" in r.data ? r.data.DiskBenchmark : null) as Benchmark | null;

export const lastQuick = (dto: TestsDto) => lastRun(dto, "MemoryQuick");
export const lastSurface = (dto: TestsDto, disk: DiskDto) => lastRun(dto, "DiskSurface", (s) => isDisk(s, disk));
/** Every read of a disk, the newest first. */
export const surfaceRuns = (dto: TestsDto, disk: DiskDto) =>
  dto.runs.filter((r) => kindOf(r) === "DiskSurface" && isDisk(r.subject, disk)).reverse();
const WORST_KEPT = 100;

function mergeRegion(a: Region, b: Region): Region {
  if (a === "Unreadable" || b === "Unreadable") return "Unreadable";
  if (a === "Unread") return b;
  if (b === "Unread") return a;
  return { Read: { worst_ms: Math.max(a.Read.worst_ms, b.Read.worst_ms) } };
}

/** A disk's reads as one map, as the Health page sees them: the newest read, extended back by each
 *  earlier one that stopped exactly where the next one started. */
export function diskSurface(dto: TestsDto, disk: DiskDto): SurfaceScan | null {
  const scans = surfaceRuns(dto, disk).map(surfaceOf).filter((s) => s !== null);
  const [newest, ...earlier] = scans;
  if (!newest) return null;
  const view: SurfaceScan = { ...newest, bands: [...newest.bands], regions: [...newest.regions], worst: [...newest.worst], unreadable: [...newest.unreadable] };
  for (const e of earlier) {
    if (e.to_bytes !== view.from_bytes || e.disk_bytes !== view.disk_bytes) break;
    view.from_bytes = e.from_bytes;
    view.bands = view.bands.map((n, i) => n + (e.bands[i] ?? 0));
    view.unreadable_blocks += e.unreadable_blocks;
    view.regions = view.regions.map((r, i) => (i < e.regions.length ? mergeRegion(r, e.regions[i]) : r));
    view.worst = [...view.worst, ...e.worst].sort((a, b) => b.ms - a.ms).slice(0, WORST_KEPT);
    view.unreadable = [...view.unreadable, ...e.unreadable.slice(0, Math.max(0, UNREADABLE_KEPT - view.unreadable.length))];
  }
  return view;
}
export const lastCheck = (dto: TestsDto, root: string) => lastRun(dto, "VolumeCheck", (s) => isVolume(s, root));
export const lastBench = (dto: TestsDto, root: string) => lastRun(dto, "DiskBenchmark", (s) => isVolume(s, root));

/** Whether a check marked a volume (`new`), found it already marked (`already`) or left it marked from before (`still`). */
export type RepairMark = "new" | "already" | "still";

/**
 * How a volume stands for the repair Windows runs at the next restart, from the newest check that
 * recorded it. A check older than this Windows session no longer counts: a boot has come since.
 */
export function repairMark(dto: TestsDto, root: string): RepairMark | null {
  const checks = dto.runs.filter((r) => kindOf(r) === "VolumeCheck" && isVolume(r.subject, root));
  for (const run of checks.reverse()) {
    const c = checkOf(run)!;
    // A check from before the bits were stored that found problems queued the repair too.
    const marked = c.dirty_after ?? (c.result === "ProblemsFound" ? true : null);
    if (marked === null) continue;
    if (!marked || run.ended_ms < dto.boot_ms) return null;
    if (c.result !== "ProblemsFound") return "still";
    return c.dirty_before === true ? "already" : "new";
  }
  return null;
}

/** The device a running test reads; a scan the helper runs for an earlier window counts too. */
export function runningDevice(dto: TestsDto, running: RunningTest | null): Device | null {
  if (running) {
    if (running.kind === "MemoryQuick") return "memory";
    const s = running.subject;
    if (typeof s === "object" && "Disk" in s) return Number(s.Disk.split(" ")[0]);
    if (typeof s === "object" && "Volume" in s) return dto.volumes.find((v) => v.root === s.Volume)?.disk ?? null;
  }
  return dto.scan?.progress.disk ?? null;
}

export interface State {
  mark: MarkKind;
  line: string;
}

export function memoryState(dto: TestsDto, running: boolean): State {
  const diag = dto.diagnostics[0] ?? null;
  const quick = lastQuick(dto);
  const cleanAt = diag?.result === "Pass" ? diag.ts_ms : 0;
  const bad = diag?.result === "Errors" || (quick?.outcome === "Problems" && quick.ended_ms > cleanAt);
  const key = running ? "running" : bad ? "errors" : diag?.result === "Pass" ? "ok" : quick ? "quick_only" : "never";
  const mark: MarkKind = running ? "Running" : bad ? "Critical" : diag?.result === "Pass" ? "Ok" : quick ? "Partial" : "Unknown";
  return { mark, line: t(`ui.tests.memory.state.${key}`) };
}

export function diskState(dto: TestsDto, disk: DiskDto, detail: DiskDetailDto | null, running: boolean): State {
  const surface = diskSurface(dto, disk);
  const checks = disk.volumes.map((v) => lastCheck(dto, v)).filter((r) => r !== null);
  const flagged = (detail?.health?.attributes ?? []).some((a) => smartWarns(a, disk.media)) || detail?.health?.predict_failure;
  const slow = surface && disk.media === "Hdd" && surface.bands[5] >= 10;
  const pick = (mark: MarkKind, key: string): State => ({ mark, line: t(`ui.tests.disk.state.${key}`) });
  if (running) return pick("Running", "running");
  if (surface?.unreadable_blocks) return pick("Critical", "unreadable");
  if (flagged || slow || checks.some((c) => c.outcome === "Problems")) return pick("Attention", "attention");
  if (surface && surface.from_bytes === 0 && surface.to_bytes >= surface.disk_bytes) return pick("Ok", "ok");
  if (surface || checks.length || detail) return pick("Partial", "partial");
  return pick("Unknown", "never");
}

/** Share of the pages the Windows diagnostic reached. */
export const coverage = (d: MemoryDiagnostic) =>
  d.detail ? d.detail.pages_tested / Math.max(1, d.detail.pages_tested + d.detail.pages_untested) : null;

export function diagnosticWord(d: MemoryDiagnostic): { mark: MarkKind; text: string } {
  if (d.result === "Errors") {
    const bad = d.detail?.bad_pages ?? 0;
    return { mark: "Critical", text: bad ? tn("ui.tests.diag.bad_pages", bad) : t("ui.tests.diag.errors") };
  }
  if (d.result === "Pass") return { mark: "Ok", text: t("ui.tests.result.clean") };
  return { mark: "Partial", text: t(`ui.tests.diag.${d.result}`) };
}

export function diagnosticCovered(d: MemoryDiagnostic, level: DetailLevel): string {
  const share = coverage(d);
  if (share === null || !d.detail) return t("ui.tests.diag.no_detail");
  if (level === "Technical")
    return t("ui.tests.diag.tech", {
      type: d.detail.test_type,
      count: d.detail.test_count,
      tested: formatNumber(d.detail.pages_tested),
      untested: formatNumber(d.detail.pages_untested),
      bad: formatNumber(d.detail.bad_pages),
    });
  if (level === "Detailed")
    return t("ui.tests.diag.detailed", {
      tested: formatNumber(d.detail.pages_tested),
      all: formatNumber(d.detail.pages_tested + d.detail.pages_untested),
      share: formatPercent(share * 100, 2),
    });
  return t("ui.tests.diag.simple", { share: formatPercent(share * 100, 2) });
}

export function outcomeMark(r: TestRunDto): MarkKind {
  return r.outcome === "Problems" ? "Critical" : r.outcome === "Interrupted" ? "Partial" : r.outcome === "Failed" ? "Unknown" : "Ok";
}

export function quickCovered(q: MemoryQuick, level: DetailLevel): string {
  const args = {
    tested: formatBytes(q.tested_bytes),
    total: formatBytes(q.total_bytes),
    share: formatPercent((q.tested_bytes / Math.max(1, q.total_bytes)) * 100),
  };
  if (level === "Simple") return t("ui.tests.quick.covered", args);
  return t("ui.tests.quick.covered.detailed", { ...args, passes: q.passes, rounds: q.rounds });
}

/** The sectors a read keeps at most; a list that reached it may have missed more. */
const UNREADABLE_KEPT = 1000;

/** "2 sectors could not be read", by sector when the read kept them, else by region. */
export function unreadableText(s: SurfaceScan): string {
  const sectors = s.unreadable.length;
  if (!sectors) return tn("ui.tests.surface.unreadable", s.unreadable_blocks);
  if (sectors >= UNREADABLE_KEPT) return t("ui.tests.surface.sectors_bad_least", { count: sectors });
  return tn("ui.tests.surface.sectors_bad", sectors);
}

export function surfaceWord(r: TestRunDto, s: SurfaceScan, disk: DiskDto): { mark: MarkKind; text: string } {
  if (s.failure && s.to_bytes === s.from_bytes) return { mark: "Unknown", text: t("ui.tests.result.failed") };
  if (s.unreadable_blocks) return { mark: "Critical", text: unreadableText(s) };
  if (r.outcome === "Interrupted")
    return { mark: "Partial", text: t("ui.tests.surface.stopped", { share: formatPercent((s.to_bytes / Math.max(1, s.disk_bytes)) * 100) }) };
  if (disk.media === "Hdd" && s.bands[5] >= 10) return { mark: "Attention", text: tn("ui.tests.surface.slow", s.bands[5]) };
  return { mark: "Ok", text: t("ui.tests.surface.clean") };
}

export const benchModeName = (m: BenchMode, level: DetailLevel) =>
  t(`ui.tests.bench.mode.${m}${level === "Technical" ? ".tech" : ""}`);

/** Roughly how long a whole read of this disk takes, from its size and kind. */
export function surfaceEstimate(disk: DiskDto): string {
  const perS = disk.media === "Hdd" ? 130e6 : disk.bus === "Nvme" ? 1500e6 : 400e6;
  const ms = ((disk.bytes ?? 0) / perS) * 1000;
  return ms ? t("ui.tests.about", { time: span(ms) }) : t("ui.tests.surface.long");
}

export function span(ms: number): string {
  if (ms < 90_000) return tn("ui.tests.span.s", Math.max(1, Math.round(ms / 1000)));
  if (ms < 90 * 60_000) return tn("ui.tests.span.min", Math.round(ms / 60_000));
  return t("ui.tests.span.h", { hours: formatNumber(ms / 3_600_000, 1) });
}

export const shareOf = (part: number, whole: number) => formatPercent((part / Math.max(1, whole)) * 100);
