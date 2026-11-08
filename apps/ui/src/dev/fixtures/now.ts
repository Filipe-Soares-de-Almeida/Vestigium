// A simulated live stream for the Now screen.
// URL: ?now=calm|busy|pressure|hung|stops&last=none, plus the base mock's link, history and fail.
import type { Channel } from "@tauri-apps/api/core";

import type {
  EpisodeKind,
  Lanes,
  LiveMessage,
  LivePoint,
  NowFrame,
  OccurrenceSummary,
  PcState,
  ProcessSample,
  Reading,
  UiError,
} from "../../lib/types";
import type { Handler } from "../mock";

type Scenario = "calm" | "busy" | "pressure" | "hung" | "stops";

const query = new URLSearchParams(location.search);
const scenario = (query.get("now") ?? "calm") as Scenario;
const firstUse = query.get("history") === "empty";
const GIB = 2 ** 30;
const MEM_TOTAL = 32 * GIB;

/** Counts what the screen asked, so a check can read `window.__live`. */
const counts = { starts: 0, stops: 0, running: false };
(window as unknown as { __live: typeof counts }).__live = counts;

let timer: ReturnType<typeof setInterval> | null = null;

// A small deterministic noise, so captures repeat.
let seed = 7;
function noise(amplitude: number): number {
  seed = (seed * 1_103_515_245 + 12_345) % 2 ** 31;
  return (seed / 2 ** 31 - 0.5) * 2 * amplitude;
}

interface Target {
  cpu: number;
  memory: number;
  disk: number;
  gpu: number;
  latency: number;
}

const CALM: Target = { cpu: 9, memory: 52, disk: 3, gpu: 5, latency: 1.1 };
const HEAVY: Record<Scenario, Target> = {
  calm: CALM,
  hung: CALM,
  stops: CALM,
  busy: { cpu: 62, memory: 61, disk: 12, gpu: 8, latency: 2.2 },
  pressure: { cpu: 98, memory: 66, disk: 9, gpu: 7, latency: 2.6 },
};
/** How many seconds before now the heavy stretch began. */
const HEAVY_FOR_S: Record<Scenario, number> = { calm: 0, hung: 0, stops: 0, busy: 200, pressure: 150 };

function clamp(value: number): number {
  return Math.min(100, Math.max(0, value));
}

function readingAt(tsMs: number, agoS: number): Reading {
  const target = agoS < HEAVY_FOR_S[scenario] ? HEAVY[scenario] : CALM;
  return {
    ts_ms: tsMs,
    cpu_pct: clamp(target.cpu + noise(target.cpu > 90 ? 2 : 4)),
    mem_used_pct: clamp(target.memory + noise(0.6)),
    disk_active_pct: clamp(target.disk + noise(2.5)),
    disk_latency_ms: Math.max(0.2, target.latency + noise(0.3)),
    gpu_pct: clamp(target.gpu + noise(2)),
    probe_lag_ms: Math.max(0.1, 0.4 + noise(0.2)),
  };
}

/** The Rust side's rule, restated for the mock: busy above 38, under pressure inside an episode. */
function laneState(load: number, open: boolean): PcState {
  return open ? "pressure" : load > 38 ? "busy" : "calm";
}

function point(reading: Reading, pressure: EpisodeKind[]): LivePoint {
  const memory = (reading.mem_used_pct ?? 0) - 35;
  const gpu = (reading.gpu_pct ?? 0) * 0.55;
  const lanes: Lanes = {
    cpu: laneState(reading.cpu_pct, pressure.includes("CpuSaturated")),
    memory: laneState(memory, pressure.includes("MemoryHigh")),
    disk: laneState(reading.disk_active_pct, pressure.some((k) => typeof k === "object")),
    gpu: laneState(gpu, false),
  };
  const load = Math.max(reading.cpu_pct, reading.disk_active_pct, gpu, memory);
  return { reading, state: laneState(load, pressure.length > 0), lanes };
}

function process(
  pid: number,
  name: string,
  cpu: number,
  memoryGib: number,
  diskMbps: number,
  gpu = 0,
): ProcessSample {
  return {
    pid,
    name,
    cpu_pct: Math.max(0, cpu + noise(cpu * 0.15)),
    private_bytes: memoryGib * GIB,
    read_bps: diskMbps * 0.3 * 2 ** 20,
    write_bps: diskMbps * 0.7 * 2 ** 20,
    gpu_pct: gpu,
  };
}

function top(): ProcessSample[] {
  const list = [
    process(4120, "chrome.exe", 2.1, 0.9, 0.2, 3),
    process(4188, "chrome.exe", 1.2, 0.6, 0.1),
    process(4302, "chrome.exe", 0.5, 0.4, 0),
    process(8812, "Discord.exe", 1.4, 0.61, 0.1, 1),
    process(1544, "MsMpEng.exe", 0.8, 0.31, 0.4),
    process(6020, "explorer.exe", 0.5, 0.18, 0),
    process(9310, "steam.exe", 0.4, 0.24, 0),
    process(7702, "Vestigium.exe", 0.1, 0.04, 0.02),
  ];
  if (scenario === "busy" || scenario === "pressure") {
    list.push(process(5566, "ffmpeg.exe", scenario === "pressure" ? 91 : 52, 0.9, 12));
  }
  return programs(list);
}

/** Every program, as the watcher lists them for the full table: the ranking plus a long quiet tail. */
const QUIET = ["svchost.exe", "RuntimeBroker.exe", "dllhost.exe", "conhost.exe", "SearchHost.exe", "sihost.exe",
  "ctfmon.exe", "audiodg.exe", "fontdrvhost.exe", "WmiPrvSE.exe", "TextInputHost.exe", "Widgets.exe",
  "OneDrive.exe", "Spotify.exe", "Code.exe", "node.exe", "python.exe", "git.exe", "WindowsTerminal.exe", "Notepad.exe"];
function everything(ranking: ProcessSample[]): ProcessSample[] {
  const tail = Array.from({ length: 140 }, (_, i) =>
    process(20_000 + i * 4, i < QUIET.length ? QUIET[i] : `helper${i}.exe`, 0.3 / (i + 1), 0.4 / (i + 2), i % 9 === 0 ? 0.05 : 0),
  );
  return [...ranking, ...tail].sort((a, b) => b.cpu_pct - a.cpu_pct);
}

/** The watcher's rule, restated for the mock: one row per program, its processes summed. */
function programs(list: ProcessSample[]): ProcessSample[] {
  const byName = new Map<string, ProcessSample>();
  for (const p of [...list].sort((a, b) => b.cpu_pct - a.cpu_pct)) {
    const sum = byName.get(p.name);
    if (!sum) {
      byName.set(p.name, { ...p });
      continue;
    }
    sum.cpu_pct += p.cpu_pct;
    sum.private_bytes += p.private_bytes;
    sum.read_bps += p.read_bps;
    sum.write_bps += p.write_bps;
    sum.gpu_pct = Math.min(100, sum.gpu_pct + p.gpu_pct);
  }
  return [...byName.values()].sort((a, b) => b.cpu_pct - a.cpu_pct);
}

const latest: OccurrenceSummary | null =
  firstUse || query.get("last") === "none"
    ? null
    : {
        id: 33,
        kind: "Freeze",
        start_ms: new Date().setHours(14, 32, 10, 0) - (new Date().getHours() < 15 ? 86_400_000 : 0),
        end_ms: new Date().setHours(14, 32, 16, 200) - (new Date().getHours() < 15 ? 86_400_000 : 0),
        severity: "Severe",
        cause: { DiskIo: { disk: "0 C:", hdd: false, process: "steam.exe" } },
      };

function refuse(kind: UiError["kind"]): Promise<never> {
  return Promise.reject({ kind, message: `mock: ${kind}` } satisfies UiError);
}

export const handlers: Record<string, Handler> = {
  process_facts: (args) => ({
    started_ms: Date.now() - 3 * 3600_000,
    threads: 61,
    processes: 9,
    path: `C:\\Program Files\\${String(args.name).replace(/\.exe$/i, "")}\\${args.name}`,
    run_as: { CurrentUser: { elevated: false } },
    signature: { Valid: { signer: "Example Software Inc." } },
  }),
  program_marks: () => ({ occurrences: [], episodes: [] }),
  // Ten minutes of rankings every 5 s, with a busy stretch in the middle.
  program_recent: () =>
    Array.from({ length: 120 }, (_, i) => ({
      ts_ms: Date.now() - (119 - i) * 5000,
      cpu_pct: 2 + (i > 50 && i < 70 ? 18 : 0) + (i % 7) * 0.4,
      private_bytes: (420 + i * 1.5) * 1048576,
      io_bps: 0,
    })),
  live_start: (args) => {
    const link = query.get("link");
    if (link === "stopped") return refuse("agent_stopped");
    if (link === "mismatch") return refuse("mismatch");
    if (timer) clearInterval(timer);
    counts.starts++;
    counts.running = true;
    const channel = args.onFrame as Channel<LiveMessage>;
    const send = (message: LiveMessage) => channel.onmessage(message);
    const startedAt = Date.now();
    const kept = firstUse ? 40 : 600;
    // The watcher sends the episode under way once it counts, from the start of the heavy stretch.
    const openFor = scenario === "pressure" && HEAVY_FOR_S.pressure >= 60 ? HEAVY_FOR_S.pressure : 0;
    const recent: LivePoint[] = [];
    for (let ago = kept; ago >= 1; ago--) {
      const pressure: EpisodeKind[] = ago < openFor ? ["CpuSaturated"] : [];
      recent.push(point(readingAt(startedAt - ago * 1000, ago), pressure));
    }
    setTimeout(() => send({ kind: "recent", points: recent }), 30);

    let ticks = 0;
    let ranking: ProcessSample[] | null = null;
    const tick = () => {
      ticks++;
      const now = Date.now();
      if (scenario === "stops" && ticks > 4) {
        send({ kind: "stopped" });
        if (timer) clearInterval(timer);
        timer = null;
        return;
      }
      const reading = readingAt(now, 0);
      const heavyFor = HEAVY_FOR_S[scenario] + ticks;
      const pressure: EpisodeKind[] = scenario === "pressure" && heavyFor >= 60 ? ["CpuSaturated"] : [];
      if (ticks === 1 || ticks % 5 === 0) ranking = top();
      const frame: NowFrame = {
        sample: {
          ts_ms: now,
          cpu_pct: reading.cpu_pct,
          dpc_pct: 0.6,
          isr_pct: 0.1,
          perf_limit_pct: 100,
          proc_perf_pct: 100,
          mem_total_bytes: MEM_TOTAL,
          mem_avail_bytes: Math.round(MEM_TOTAL * (1 - (reading.mem_used_pct ?? 0) / 100)),
          commit_bytes: 20 * GIB,
          commit_limit_bytes: 48 * GIB,
          hard_faults_per_s: 0,
          gpu_pct: reading.gpu_pct,
          probe_max_lag_ms: reading.probe_lag_ms,
          sampler_lag_ms: 1,
          cores: [],
          disks: [
            {
              name: "0 C:",
              active_pct: reading.disk_active_pct,
              latency_ms: reading.disk_latency_ms,
              queue_len: 0,
              bytes_per_s: 1_000_000,
            },
          ],
        },
        top: ticks < 3 && firstUse ? null : { ts_ms: now, entries: ranking ?? [] },
        programs: ticks < 3 && firstUse ? null : { ts_ms: now, entries: everything(ranking ?? []) },
        hung: scenario === "hung" && ticks >= 3 ? [{ pid: 8812, name: "Discord.exe" }] : [],
        latest,
        pressure,
      };
      send({ kind: "now", point: point(reading, pressure), frame });
    };
    setTimeout(tick, 60);
    timer = setInterval(tick, 1000);
    return null;
  },
  live_stop: () => {
    if (timer) clearInterval(timer);
    timer = null;
    counts.stops++;
    counts.running = false;
    return null;
  },
};
