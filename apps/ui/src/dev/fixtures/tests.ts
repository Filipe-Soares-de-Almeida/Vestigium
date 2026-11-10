// Tests page data. URL: ?tests=none|some|errors|running (default some) &memory=low &scan=1
// Every disk, size and result here is invented; nothing was read from a real PC.
import { emit } from "@tauri-apps/api/event";

import type {
  BenchMode,
  DiskDetailDto,
  MemoryDiagnostic,
  Pattern,
  Region,
  RunningTest,
  SurfaceScan,
  TestFinished,
  TestRun,
  TestRunDto,
  TestsDto,
} from "../../lib/types";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const scenario = query.get("tests") ?? "some";
const DAY = 24 * 60 * 60_000;
const GIB = 2 ** 30;
const MIB = 2 ** 20;
const now = Date.now();

const range = (low: number, high: number) => ({ low, high });
const NVME = (
  [["SeqRead", range(1500, 7500)], ["SeqWrite", range(1000, 7000)], ["RandRead32", range(300, 3000)], ["RandWrite32", range(300, 3000)], ["RandRead1", range(40, 100)], ["RandWrite1", range(100, 300)]] as const
).map(([mode, r]) => ({ mode, ...r }));
const HDD = (
  [["SeqRead", range(80, 250)], ["SeqWrite", range(80, 250)], ["RandRead32", range(0.5, 3)], ["RandWrite32", range(0.5, 3)], ["RandRead1", range(0.3, 1.5)], ["RandWrite1", range(0.3, 3)]] as const
).map(([mode, r]) => ({ mode, ...r }));

const PATTERNS: Pattern[] = ["OwnAddress", "Random", "Compare", "SolidBits", "Checkerboard", "WalkingOnes", "WalkingZeros"];
const MODES: BenchMode[] = ["SeqRead", "SeqWrite", "RandRead32", "RandWrite32", "RandRead1", "RandWrite1"];

function regions(slowAt: number[], unreadableAt: number[], readUpTo = 1024): Region[] {
  return Array.from({ length: 1024 }, (_, i): Region => {
    if (i >= readUpTo) return "Unread";
    if (unreadableAt.includes(i)) return "Unreadable";
    const worst = slowAt.includes(i) ? 640 + i % 300 : 4 + ((i * 37) % 11);
    return { Read: { worst_ms: worst } };
  });
}

function surface(errors: boolean): SurfaceScan {
  const disk = 2000 * GIB;
  const blocks = disk / MIB;
  return {
    disk: 1,
    media: "Hdd",
    disk_bytes: disk,
    sector_bytes: 4096,
    from_bytes: 0,
    to_bytes: disk,
    bands: errors ? [blocks - 9000, 7200, 1500, 260, 28, 12] : [blocks - 6000, 5100, 820, 80, 0, 0],
    unreadable_blocks: errors ? 2 : 0,
    regions: regions(errors ? [311, 312, 760] : [], errors ? [312, 861] : []),
    worst: errors
      ? [
          { offset: 650 * GIB, ms: 2140 },
          { offset: 651 * GIB, ms: 1310 },
          { offset: 1580 * GIB, ms: 702 },
        ]
      : [{ offset: 120 * GIB, ms: 131 }],
    unreadable: errors ? [651 * GIB + 8192, 1720 * GIB + 4096] : [],
    failure: null,
  };
}

function run(id: number, daysAgo: number, minutes: number, partial: Omit<TestRun, "started_ms" | "ended_ms">): TestRunDto {
  const ended = now - daysAgo * DAY;
  return { id, started_ms: ended - minutes * 60_000, ended_ms: ended, ...partial };
}

/** An earlier read of the same disk that was stopped at 40%. */
function stoppedSurface(): SurfaceScan {
  const scan = surface(false);
  return { ...scan, to_bytes: scan.disk_bytes * 0.4, regions: regions([200], [], 410), worst: [] };
}

function runs(errors: boolean): TestRunDto[] {
  return [
    run(20, 90, 110, { subject: { Disk: "1 D:" }, outcome: "Interrupted", data: { DiskSurface: stoppedSurface() } }),
    run(1, 40, 270, { subject: { Disk: "1 D:" }, outcome: errors ? "Problems" : "Clean", data: { DiskSurface: surface(errors) } }),
    run(2, 12, 1, {
      subject: { Volume: "C:\\" },
      outcome: "Clean",
      data: {
        DiskBenchmark: {
          volume: "C:\\",
          bus: "Nvme",
          media: "Ssd",
          file_bytes: GIB,
          written_bytes: 1.31 * GIB,
          results: [
            { mode: "SeqRead", mb_per_s: 3120, iops: 3120, max_ms: 9.1 },
            { mode: "SeqWrite", mb_per_s: 2480, iops: 2480, max_ms: 14.2 },
            { mode: "RandRead32", mb_per_s: 880, iops: 225_000, max_ms: 2.2 },
            { mode: "RandWrite32", mb_per_s: 610, iops: 156_000, max_ms: 3.9 },
            { mode: "RandRead1", mb_per_s: 61, iops: 15_600, max_ms: 1.1 },
            { mode: "RandWrite1", mb_per_s: 172, iops: 44_000, max_ms: 1.8 },
          ],
          outside_io: false,
          failure: null,
        },
      },
    }),
    run(3, 6, 4, {
      subject: { Volume: errors ? "D:\\" : "C:\\" },
      outcome: errors ? "Problems" : "Clean",
      data: {
        VolumeCheck: {
          volume: errors ? "D:\\" : "C:\\",
          exit_code: errors ? 3 : 0,
          result: errors ? "ProblemsFound" : "NoProblems",
          output: "Stage 1: Examining basic file system structure ...\r\nWindows has scanned the file system.",
          dirty_before: false,
          dirty_after: errors,
        },
      },
    }),
    run(4, 1, 5, {
      subject: "Pc",
      outcome: errors ? "Problems" : "Clean",
      data: {
        MemoryQuick: {
          total_bytes: 32 * GIB,
          tested_bytes: 10.5 * GIB,
          rounds: 3,
          passes: 24,
          error_count: errors ? 3 : 0,
          errors: errors
            ? [
                { offset: 3.2 * GIB, pattern: "WalkingOnes", expected: 2 ** 20, actual: 2 ** 20 + 32 },
                { offset: 3.2 * GIB, pattern: "Random", expected: 0x1234, actual: 0x1214 },
                { offset: 3.2 * GIB + 8, pattern: "Checkerboard", expected: 0x5555, actual: 0x5575 },
              ]
            : [],
          failure: null,
        },
      },
    }),
  ];
}

function diagnostics(errors: boolean): MemoryDiagnostic[] {
  const detail = {
    memory_mb: 32_713,
    test_type: 10,
    duration_s: 2426,
    test_count: 12,
    pages_tested: 8_319_731,
    pages_untested: 1843,
    bad_pages: errors ? 6 : 0,
  };
  return [{ record_id: 48_211, ts_ms: now - 20 * DAY, result: errors ? "Errors" : "Pass", detail }];
}

let running: RunningTest | null = null;
let timer: ReturnType<typeof setInterval> | null = null;

function dto(): TestsDto {
  const none = scenario === "none";
  const errors = scenario === "errors";
  return {
    memory: {
      total_bytes: 32 * GIB,
      available_bytes: query.get("memory") === "low" ? 2.5 * GIB : 21 * GIB,
      quick_bytes: query.get("memory") === "low" ? null : 10.5 * GIB,
      minutes: [1, 5, 15],
    },
    diagnostics: none ? [] : diagnostics(errors),
    disks: [
      { number: 0, name: "0 C:", media: "Ssd", bus: "Nvme", bytes: 1000 * GIB, volumes: ["C:\\"], ranges: NVME },
      { number: 1, name: "1 D:", media: "Hdd", bus: "Sata", bytes: 2000 * GIB, volumes: ["D:\\"], ranges: HDD },
    ],
    volumes: [
      { root: "C:\\", total_bytes: 930 * GIB, free_bytes: 410 * GIB, system: true, disk: 0, file_system: "NTFS" },
      { root: "D:\\", total_bytes: 1863 * GIB, free_bytes: 1200 * GIB, system: false, disk: 1, file_system: "NTFS" },
    ],
    runs: none ? [] : runs(errors),
    running,
    scan:
      query.get("scan") === "1" && !running
        ? {
            stem: String(now - 600_000),
            progress: {
              disk: 1,
              ts_ms: now,
              at_bytes: 310 * GIB,
              from_bytes: 0,
              disk_bytes: 2000 * GIB,
              bands: [300_000, 15_000, 2000, 120, 4, 0],
              unreadable_blocks: 0,
            },
          }
        : null,
    bench_file_bytes: GIB,
    bench_needed_bytes: GIB + GIB / 2,
  };
}

function finish(finished: TestFinished): void {
  if (timer) clearInterval(timer);
  timer = null;
  running = null;
  void emit("test-finished", finished);
}

function startMemory(minutes: number): RunningTest {
  running = { kind: "MemoryQuick", subject: "Pc", started_ms: Date.now(), ends_ms: Date.now() + minutes * 60_000 };
  let passes = 0;
  timer = setInterval(() => {
    passes += 1;
    void emit("test-progress", {
      kind: "memory",
      pattern: PATTERNS[passes % PATTERNS.length],
      rounds: Math.floor(passes / PATTERNS.length),
      passes,
      error_count: 0,
    });
    if (passes >= 14) finish({ id: 99, run: null, exit_code: null });
  }, 600);
  return running;
}

function startBench(volume: string): RunningTest {
  running = { kind: "DiskBenchmark", subject: { Volume: volume }, started_ms: Date.now(), ends_ms: null };
  let step = 0;
  timer = setInterval(() => {
    if (step < MODES.length) void emit("test-progress", { kind: "benchmark", mode: MODES[step] });
    step += 1;
    if (step > MODES.length) finish({ id: 98, run: null, exit_code: null });
  }, 900);
  return running;
}

function startSurface(disk: number): RunningTest {
  running = { kind: "DiskSurface", subject: { Disk: String(disk) }, started_ms: Date.now(), ends_ms: null };
  const size = 2000 * GIB;
  let at = 0;
  timer = setInterval(() => {
    at = Math.min(size, at + 160 * GIB);
    const blocks = at / MIB;
    void emit("test-progress", {
      kind: "surface",
      disk,
      ts_ms: Date.now(),
      at_bytes: at,
      from_bytes: 0,
      disk_bytes: size,
      bands: [blocks * 0.95, blocks * 0.04, blocks * 0.009, blocks * 0.001, 0, 0],
      unreadable_blocks: 0,
    });
    if (at >= size) finish({ id: 97, run: null, exit_code: 0 });
  }, 700);
  return running;
}

function detail(args: Record<string, unknown>): DiskDetailDto {
  const number = args.number as number;
  if (number === 0) {
    const health = { critical_warning: 0, temperature_c: 41, available_spare_pct: 100, spare_threshold_pct: 10, percentage_used: 3, media_errors: 0 };
    return {
      health: { name: "0 C:", read_ms: Date.now(), predict_failure: false, attributes: [], nvme: health },
      nvme: {
        health,
        data_read_bytes: 41.2e12,
        data_written_bytes: 28.7e12,
        power_cycles: 1840,
        power_on_hours: 9120,
        unsafe_shutdowns: 61,
        error_log_entries: 0,
      },
    };
  }
  const attr = (id: number, current: number, worst: number, raw: number) => ({ id, current, worst, raw });
  return {
    health: {
      name: "1 D:",
      read_ms: Date.now(),
      predict_failure: false,
      attributes: [
        attr(1, 200, 200, 0),
        attr(3, 178, 175, 6075),
        attr(4, 100, 100, 1520),
        attr(5, 200, 200, scenario === "errors" ? 8 : 0),
        attr(9, 61, 61, 28_840),
        attr(12, 100, 100, 1511),
        attr(194, 113, 98, 37),
        attr(197, 200, 200, scenario === "errors" ? 2 : 0),
        attr(198, 100, 253, 0),
        attr(199, 200, 200, 0),
      ],
      nvme: null,
    },
    nvme: null,
  };
}

if (scenario === "running") startMemory(5);

export const handlers: Record<string, Handler> = {
  tests: () => dto(),
  disk_detail: detail,
  // Thirty daily readings that creep up for the watched attributes and stay put for the others.
  smart_history: (args) =>
    Array.from({ length: 30 }, (_, i) => ({
      read_ms: now - (29 - i) * DAY,
      current: 100,
      worst: 100,
      raw: [5, 197].includes(args.attr as number) && scenario === "errors" ? Math.floor(i / 6) : 1000 + i * 24,
    })),
  open_memory_diagnostic: () => null,
  start_memory_test: (args) => startMemory(args.minutes as number),
  start_benchmark: (args) => startBench(args.volume as string),
  start_surface_scan: (args) => startSurface(args.disk as number),
  start_volume_check: (args) => {
    running = { kind: "VolumeCheck", subject: { Volume: args.volume as string }, started_ms: Date.now(), ends_ms: null };
    timer = setInterval(() => finish({ id: 96, run: null, exit_code: 0 }), 2500);
    return running;
  },
  stop_test: () => {
    if (running) finish({ id: 95, run: null, exit_code: 0 });
    return null;
  },
  test_running: () => running,
};
