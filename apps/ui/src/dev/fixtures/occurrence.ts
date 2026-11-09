// Eight occurrences for the report in the browser, #/occurrence/1 to 8 (8 is a fall), all today; any other id is gone.
import type {
  Cause,
  Columns,
  CrashKind,
  DiskLoad,
  Evidence,
  Fact,
  FactKind,
  Gap,
  HangSpan,
  MachineInfo,
  Metric,
  OccurrenceDto,
  OccurrenceKind,
  PcState,
  ProcessSample,
  ProcessTop,
  Sample,
  Severity,
  Suspect,
  Trigger,
  WinEvent,
} from "../../lib/types";
import type { Handler } from "../mock";

const MB = 1024 ** 2;
const GB = 1024 ** 3;
const LOGICAL = 8;
const MEM_TOTAL = 32 * GB;
const MACHINE: MachineInfo = {
  logical_cpus: LOGICAL,
  mem_total_bytes: MEM_TOTAL,
  disks: [
    { name: "0 C:", media: "Ssd" },
    { name: "1 D:", media: "Hdd" },
  ],
};

const midnight = new Date();
midnight.setHours(0, 0, 0, 0);
const at = (h: number, m: number, s: number) => midnight.getTime() + ((h * 60 + m) * 60 + s) * 1000;

interface Disk {
  active: number;
  latency: number;
  queue: number;
  bps: number;
}

/** One second of readings; anything left out is calm. */
interface Moment {
  cpu: number;
  mem: number;
  c?: Partial<Disk>;
  d?: Partial<Disk>;
  gpu?: number;
  lag?: number;
  faults?: number;
  dpc?: number;
}

interface Spec {
  id: number;
  start: number;
  end: number;
  kind: OccurrenceKind;
  severity: Severity;
  triggers: Trigger[];
  suspects: Suspect[];
  facts: [number, FactKind][];
  /** Readings at a second from the start; `null` where none was taken. */
  moment: (rel: number) => Moment | null;
  /** The ranking at a second from the start, taken every 2 s. */
  procs: (rel: number) => ProcessSample[];
  events?: WinEvent[];
  hangs?: HangSpan[];
  gaps?: Gap[];
  sensitive?: boolean;
  truncated?: boolean;
}

/** Repeatable noise, so the screens look the same on every load. */
function noise(seed: number): (spread: number) => number {
  let state = seed >>> 0;
  return (spread) => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return ((state / 2 ** 32) * 2 - 1) * spread;
  };
}

const pct = (v: number) => Math.min(100, Math.max(0, v));

function disk(name: string, d: Partial<Disk> = {}): DiskLoad {
  return {
    name,
    active_pct: pct(d.active ?? 2),
    latency_ms: d.latency ?? 1.5,
    queue_len: d.queue ?? 0,
    bytes_per_s: d.bps ?? 200_000,
  };
}

function sample(ts_ms: number, m: Moment): Sample {
  const cpu = pct(m.cpu);
  const dpc = m.dpc ?? 0.8;
  return {
    ts_ms,
    cpu_pct: cpu,
    dpc_pct: dpc / LOGICAL,
    isr_pct: 0.05,
    perf_limit_pct: 100,
    proc_perf_pct: 100,
    mem_total_bytes: MEM_TOTAL,
    mem_avail_bytes: Math.round((MEM_TOTAL * (100 - pct(m.mem))) / 100),
    commit_bytes: Math.round((MEM_TOTAL * pct(m.mem)) / 90),
    commit_limit_bytes: 48 * GB,
    hard_faults_per_s: m.faults ?? 12,
    gpu_pct: m.gpu ?? 3,
    probe_max_lag_ms: m.lag ?? 1.2,
    sampler_lag_ms: 0.8,
    cores: Array.from({ length: LOGICAL }, (_, i) => ({
      cpu_pct: cpu,
      dpc_pct: i === 0 ? dpc * 0.7 : 0.2,
      isr_pct: i === 0 ? dpc * 0.3 : 0.1,
    })),
    disks: [disk("0 C:", m.c), disk("1 D:", m.d)],
  };
}

function proc(pid: number, name: string, cpu: number, mem: number, io: number, gpu = 0): ProcessSample {
  return {
    pid,
    name,
    cpu_pct: cpu,
    private_bytes: mem,
    read_bps: io * 0.4,
    write_bps: io * 0.6,
    gpu_pct: gpu,
  };
}

/** The chart columns the window's Rust side sends: one point a second, `null` where nothing was read. */
function columns(samples: Sample[], fromMs: number, toMs: number): Columns {
  const start = fromMs - (fromMs % 1000);
  const out: Columns = {
    t: [],
    cpu: [],
    mem_used_pct: [],
    disk_active_max: [],
    disk_latency_max: [],
    gpu: [],
    probe_lag: [],
    state: [],
  };
  for (let s = start; s <= toMs; s += 1000) {
    const found = samples.find((x) => x.ts_ms >= s && x.ts_ms < s + 1000);
    out.t.push(s / 1000);
    const mem = found ? 100 - (found.mem_avail_bytes / found.mem_total_bytes) * 100 : null;
    const active = found ? Math.max(...found.disks.map((d) => d.active_pct)) : null;
    out.cpu.push(found?.cpu_pct ?? null);
    out.mem_used_pct.push(mem);
    out.disk_active_max.push(active);
    out.disk_latency_max.push(found ? Math.max(...found.disks.map((d) => d.latency_ms)) : null);
    out.gpu.push(found?.gpu_pct ?? null);
    out.probe_lag.push(found?.probe_max_lag_ms ?? null);
    const load = found ? Math.max(found.cpu_pct, active ?? 0, (found.gpu_pct ?? 0) * 0.55, (mem ?? 0) - 35) : 0;
    const state: PcState = !found ? "suspended" : load > 38 ? "busy" : "calm";
    out.state.push(state);
  }
  return out;
}

function build(spec: Spec): OccurrenceDto {
  const from = spec.start - 60_000;
  const to = spec.end + 30_000;
  const samples: Sample[] = [];
  const tops: ProcessTop[] = [];
  for (let rel = -60; spec.start + rel * 1000 <= to; rel += 1) {
    const m = spec.moment(rel);
    if (!m) continue;
    const ts = spec.start + rel * 1000;
    samples.push(sample(ts, m));
    if (rel % 2 === 0) tops.push({ ts_ms: ts, entries: spec.procs(rel) });
  }
  const facts: Fact[] = spec.facts
    .map(([offset, kind]) => ({ at_ms: spec.start + offset, kind }))
    .sort((a, b) => a.at_ms - b.at_ms);
  return {
    id: spec.id,
    kind: spec.kind,
    start_ms: spec.start,
    end_ms: spec.end,
    severity: spec.severity,
    triggers: spec.triggers,
    window: {
      from_ms: from,
      to_ms: to,
      samples,
      tops,
      events: spec.events ?? [],
      hangs: spec.hangs ?? [],
      gaps: spec.gaps ?? [],
      machine: MACHINE,
    },
    suspects: spec.suspects,
    facts,
    sensitive: spec.sensitive ?? false,
    truncated: spec.truncated ?? false,
    chart: columns(samples, from, to),
    capture: null,
    tests: [],
  };
}

function event(ts_ms: number, provider: string, id: number, meaning: WinEvent["meaning"], record: number): WinEvent {
  const channel = meaning === "AppHang" || meaning === "AppCrash" ? "Application" : "System";
  return { ts_ms, channel, provider, id, record_id: record, level: 3, meaning, data_xml: "<EventData/>" };
}

function m(ts_ms: number, metric: Metric, value: number, threshold: number, subject: string | null = null): Evidence {
  return { Metric: { ts_ms, metric, value, threshold, subject } };
}

function suspect(cause: Cause, confidence: Suspect["confidence"], evidence: Evidence[], next: Suspect["next_steps"]): Suspect {
  return { cause, confidence, evidence, next_steps: next };
}

const steady = (seed: number, cpu: number, memUsed: number) => {
  const n = noise(seed);
  return (): Moment => ({ cpu: cpu + n(3), mem: memUsed + n(0.6), c: { active: 4 + n(2), latency: 1.6 + n(0.4) } });
};

/** A severe freeze while disk C: stalled and Windows reset its controller. */
function storageFreeze(): OccurrenceDto {
  const start = at(14, 32, 18);
  const n = noise(1);
  const reset = event(start + 1000, "storahci", 129, "StorageReset", 88121);
  const retry = event(start + 2000, "disk", 153, "DiskRetry", 88122);
  return build({
    id: 1,
    start,
    end: start + 6800,
    kind: "Freeze",
    severity: "Severe",
    triggers: [{ Freeze: { start_ms: start, end_ms: start + 6800, lag_ms: 6812 } }],
    moment: (rel) => {
      if (rel >= 0 && rel < 7) return null;
      const base = { cpu: 14 + n(4), mem: 61 + n(0.8) };
      if (rel < -26) return { ...base, c: { active: 5 + n(3), latency: 1.8 + n(0.5), bps: 3 * MB } };
      if (rel < -9) {
        return { ...base, c: { active: 72 + n(10), latency: 6 + (rel + 26) * 1.9, queue: 2, bps: 205 * MB } };
      }
      if (rel < 0) {
        const k = (rel + 9) / 8;
        return { ...base, c: { active: 100, latency: 480 + k * 1660, queue: Math.round(22 + k * 9), bps: 40 * MB } };
      }
      if (rel === 7) return { ...base, lag: 6812, c: { active: 96, latency: 1200, queue: 10 } };
      return { ...base, c: { active: 8 + n(3), latency: 3 + n(0.8), bps: 2 * MB } };
    },
    procs: (rel) => [
      proc(9120, "Launcher.exe", 6.2, 410 * MB, rel >= -26 && rel < 0 ? 205 * MB : 1 * MB),
      proc(4412, "Browser.exe", 4.1, 3.2 * GB, 1 * MB),
      proc(3876, "MsMpEng.exe", 2.3, 220 * MB, 12 * MB),
      proc(7312, "Chatter.exe", 0.9, 498 * MB, 0),
      proc(5520, "explorer.exe", 0.4, 98 * MB, 0),
    ],
    events: [reset, retry],
    hangs: [{ pid: 7312, name: "Chatter.exe", start_ms: start + 8000, end_ms: start + 11000, outcome: "Recovered" }],
    suspects: [
      suspect({ Storage: { disk: "0 C:" } }, "High", [
        { Event: { ts_ms: reset.ts_ms, provider: "storahci", id: 129, meaning: "StorageReset" } },
        { Event: { ts_ms: retry.ts_ms, provider: "disk", id: 153, meaning: "DiskRetry" } },
        m(start - 1000, "DiskLatencyMs", 2140, 200, "0 C:"),
      ], ["UpdateStorageDriver", "CheckDiskCable", "RunDiskTest"]),
      suspect({ DiskIo: { disk: "0 C:", hdd: false, process: "Launcher.exe" } }, "Medium", [
        m(start - 1000, "DiskLatencyMs", 2140, 200, "0 C:"),
        m(start - 1000, "DiskQueue", 31, 8, "0 C:"),
        m(start - 5000, "DiskActivePct", 100, 90, "0 C:"),
        { Process: { pid: 9120, name: "Launcher.exe", metric: "IoBytesPerS", value: 162 * MB } },
      ], ["FindDiskHeavyApp"]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "CpuPct", name: "Launcher.exe", value: 6.2 } }],
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Browser.exe", value: 3.2 * GB } }],
      [-26_000, { Busiest: { metric: "IoBytesPerS", name: "Launcher.exe", value: 205 * MB } }],
      [-9000, { Crossed: { metric: "DiskLatencyMs", value: 2140, threshold: 200, subject: "0 C:" } }],
      [-9000, { Crossed: { metric: "DiskQueue", value: 31, threshold: 8, subject: "0 C:" } }],
      [0, { FreezeStarted: { duration_ms: 6812 } }],
      [1000, { Event: { meaning: "StorageReset", provider: "storahci", id: 129 } }],
      [2000, { Event: { meaning: "DiskRetry", provider: "disk", id: 153 } }],
      [6800, "FreezeEnded"],
      [8000, { AppHung: { name: "Chatter.exe" } }],
      [11_000, { AppRecovered: { name: "Chatter.exe" } }],
    ],
  });
}

/** Chatter stopped answering for 12 s while the rest of the PC was calm. */
function appHang(): OccurrenceDto {
  const start = at(9, 14, 7);
  const calm = steady(2, 9, 55);
  const span: HangSpan = { pid: 7312, name: "Chatter.exe", start_ms: start, end_ms: start + 12_000, outcome: "Recovered" };
  return build({
    id: 2,
    start,
    end: start + 12_000,
    kind: "AppHang",
    severity: "Severe",
    triggers: [{ AppHang: span }],
    moment: calm,
    procs: () => [
      proc(7312, "Chatter.exe", 0.4, 512 * MB, 0),
      proc(4412, "Browser.exe", 3.1, 2.8 * GB, 120 * 1024),
      proc(5520, "explorer.exe", 0.8, 96 * MB, 0),
    ],
    hangs: [span],
    suspects: [
      suspect({ App: { name: "Chatter.exe" } }, "High", [
        { Hang: { name: "Chatter.exe", duration_ms: 12_000 } },
        { Process: { pid: 7312, name: "Chatter.exe", metric: "CpuPct", value: 0.4 } },
        { Process: { pid: 7312, name: "Chatter.exe", metric: "IoBytesPerS", value: 0 } },
      ], ["ReportToAppVendor"]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "CpuPct", name: "Browser.exe", value: 3.1 } }],
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Browser.exe", value: 2.8 * GB } }],
      [-60_000, { Busiest: { metric: "IoBytesPerS", name: "Browser.exe", value: 120 * 1024 } }],
      [0, { AppHung: { name: "Chatter.exe" } }],
      [12_000, { AppRecovered: { name: "Chatter.exe" } }],
    ],
  });
}

/** A medium freeze while the hard disk D: was at 100% for a minute; the busiest program is only probable. */
function diskEpisode(): OccurrenceDto {
  const start = at(11, 2, 40);
  const n = noise(3);
  return build({
    id: 3,
    start,
    end: start + 1900,
    kind: "Freeze",
    severity: "Medium",
    triggers: [{ Freeze: { start_ms: start, end_ms: start + 1900, lag_ms: 1904 } }],
    moment: (rel) => {
      if (rel >= 0 && rel < 2) return null;
      const base = { cpu: 18 + n(5), mem: 58 + n(0.6), dpc: rel >= -3 && rel < 0 ? 23 : 1.2, lag: rel === 2 ? 1904 : undefined };
      if (rel < -50) return { ...base, d: { active: 12 + n(4), latency: 9 + n(2), queue: 1, bps: 4 * MB } };
      const wave = rel === -3 ? 912 : 520 + 280 * Math.sin(rel / 4) + n(60);
      return {
        ...base,
        d: { active: 99 + n(1), latency: wave, queue: rel === -3 ? 19 : 12 + Math.round(n(3) + 3), bps: 24 * MB },
      };
    },
    procs: (rel) => [
      proc(6204, "Browser.exe", 6, 1.4 * GB, rel >= -50 ? 25 * MB : 2 * MB),
      proc(9930, "Editor.exe", 3.2, 1.8 * GB, 0.2 * MB),
      proc(3344, "SearchIndexer.exe", 2, 120 * MB, 9 * MB),
      proc(8812, "CloudSync.exe", 1.5, 210 * MB, 6 * MB),
    ],
    suspects: [
      suspect({ DiskIo: { disk: "1 D:", hdd: true, process: "Browser.exe" } }, "High", [
        m(start - 3000, "DiskLatencyMs", 912, 200, "1 D:"),
        m(start - 3000, "DiskQueue", 19, 8, "1 D:"),
        m(start - 8000, "DiskActivePct", 100, 90, "1 D:"),
        { Process: { pid: 6204, name: "Browser.exe", metric: "IoBytesPerS", value: 24.6 * MB } },
      ], ["FindDiskHeavyApp", "RunDiskTest"]),
      suspect({ Driver: { core: 0 } }, "Low", [m(start - 2000, "CoreDpcIsrPct", 23, 20, "0")], ["DeepCapture"]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "CpuPct", name: "Browser.exe", value: 6 } }],
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Editor.exe", value: 1.8 * GB } }],
      [-50_000, { Busiest: { metric: "IoBytesPerS", name: "Browser.exe", value: 25 * MB } }],
      [-50_000, { Crossed: { metric: "DiskLatencyMs", value: 912, threshold: 200, subject: "1 D:" } }],
      [-50_000, { Crossed: { metric: "DiskQueue", value: 19, threshold: 8, subject: "1 D:" } }],
      [-3000, { Crossed: { metric: "CoreDpcIsrPct", value: 23, threshold: 20, subject: "0" } }],
      [0, { FreezeStarted: { duration_ms: 1904 } }],
      [1900, "FreezeEnded"],
    ],
  });
}

/** A light freeze with memory almost full, in sensitive mode; watching was paused 20 s later. */
function memoryEpisode(): OccurrenceDto {
  const start = at(16, 10, 5);
  const n = noise(4);
  return build({
    id: 4,
    start,
    end: start + 600,
    kind: "Freeze",
    severity: "Light",
    triggers: [{ Freeze: { start_ms: start, end_ms: start + 600, lag_ms: 640 } }],
    moment: (rel) => {
      if (rel === 0 || rel > 20) return null;
      const mem = rel < -30 ? 88 + (rel + 60) * 0.25 : 96.6 + n(0.4);
      return {
        cpu: 22 + n(5),
        mem,
        faults: rel < -30 ? 150 + n(40) : 2380 + n(300),
        lag: rel === 1 ? 640 : undefined,
        c: { active: 45 + n(10), latency: 25 + n(8), queue: 2, bps: 60 * MB },
      };
    },
    procs: () => [
      proc(4412, "Browser.exe", 3.5, 9.1 * GB, 200 * 1024),
      proc(9930, "Editor.exe", 1.2, 1.8 * GB, 0),
      proc(3876, "MsMpEng.exe", 2, 230 * MB, 1 * MB),
      proc(7312, "Chatter.exe", 0.6, 520 * MB, 0),
    ],
    gaps: [{ start_ms: start + 21_000, end_ms: start + 120_000, reason: "Paused" }],
    sensitive: true,
    truncated: true,
    suspects: [
      suspect({ Memory: { process: "Browser.exe" } }, "Medium", [
        m(start - 4000, "MemAvailPct", 3.1, 5),
        m(start - 2000, "HardFaultsPerS", 2680, 1000),
        { Process: { pid: 4412, name: "Browser.exe", metric: "PrivateBytes", value: 9.1 * GB } },
      ], ["CloseMemoryHeavyApp"]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "CpuPct", name: "Browser.exe", value: 3.5 } }],
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Browser.exe", value: 9.1 * GB } }],
      [-60_000, { Busiest: { metric: "IoBytesPerS", name: "MsMpEng.exe", value: 1 * MB } }],
      [-30_000, { Crossed: { metric: "HardFaultsPerS", value: 2680, threshold: 1000, subject: null } }],
      [-30_000, { Crossed: { metric: "MemAvailPct", value: 3.1, threshold: 5, subject: null } }],
      [0, { FreezeStarted: { duration_ms: 640 } }],
      [600, "FreezeEnded"],
      [21_000, { Gap: { reason: "Paused" } }],
    ],
  });
}

/** Windows reset the graphics driver without the PC itself stalling. */
function reported(): OccurrenceDto {
  const start = at(19, 5, 46);
  const n = noise(5);
  const tdr = event(start, "Display", 4101, "DisplayReset", 90231);
  return build({
    id: 5,
    start,
    end: start,
    kind: "Reported",
    severity: "Light",
    triggers: [{ Reported: { event: tdr } }],
    moment: (rel) => ({ cpu: 35 + n(6), mem: 70 + n(0.5), gpu: rel >= -40 ? 100 : 92 + n(3) }),
    procs: () => [
      proc(11220, "Starfall.exe", 28, 6.2 * GB, 3 * MB, 97),
      proc(1312, "dwm.exe", 2, 140 * MB, 0, 4),
      proc(7312, "Chatter.exe", 1.2, 520 * MB, 0, 2),
    ],
    events: [tdr],
    suspects: [
      suspect("GpuDriver", "High", [
        { Event: { ts_ms: start, provider: "Display", id: 4101, meaning: "DisplayReset" } },
        m(start - 40_000, "GpuPct", 100, 95),
      ], ["UpdateGpuDriver"]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "CpuPct", name: "Starfall.exe", value: 28 } }],
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Starfall.exe", value: 6.2 * GB } }],
      [-60_000, { Busiest: { metric: "IoBytesPerS", name: "Starfall.exe", value: 3 * MB } }],
      [-40_000, { Crossed: { metric: "GpuPct", value: 100, threshold: 95, subject: null } }],
      [0, { Event: { meaning: "DisplayReset", provider: "Display", id: 4101 } }],
    ],
  });
}

/** A short freeze with nothing on the PC out of the ordinary. */
function unclear(): OccurrenceDto {
  const start = at(21, 47, 3);
  const calm = steady(6, 12, 52);
  return build({
    id: 6,
    start,
    end: start + 400,
    kind: "Freeze",
    severity: "Light",
    triggers: [{ Freeze: { start_ms: start, end_ms: start + 400, lag_ms: 412 } }],
    moment: (rel) => (rel === 0 ? null : { ...calm(), lag: rel === 1 ? 412 : undefined }),
    procs: () => [proc(4412, "Browser.exe", 4, 2.6 * GB, 300 * 1024), proc(5520, "explorer.exe", 0.6, 96 * MB, 0)],
    suspects: [
      suspect("Unclear", "Low", [
        m(start - 12_000, "CpuPct", 19, 95),
        m(start - 3000, "MemAvailPct", 47.6, 5),
        m(start - 7000, "DiskLatencyMs", 4.2, 200, "0 C:"),
      ], ["EnableSensitiveMode", "DeepCapture"]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "CpuPct", name: "Browser.exe", value: 4 } }],
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Browser.exe", value: 2.6 * GB } }],
      [-60_000, { Busiest: { metric: "IoBytesPerS", name: "Browser.exe", value: 300 * 1024 } }],
      [0, { FreezeStarted: { duration_ms: 412 } }],
      [400, "FreezeEnded"],
    ],
  });
}

/** Seven stutters in sensitive mode while a video conversion held every core. */
function stutters(): OccurrenceDto {
  const start = at(12, 40, 10);
  const n = noise(7);
  return build({
    id: 7,
    start,
    end: start + 8000,
    kind: "Stutter",
    severity: "Light",
    triggers: [{ Stutter: { start_ms: start, end_ms: start + 8000, count: 7, worst_ms: 230 } }],
    moment: (rel) => ({
      cpu: rel >= -20 && rel < 40 ? 98 + n(2) : 14 + n(4),
      mem: 63 + n(0.5),
      lag: rel >= 0 && rel <= 8 ? 60 + Math.abs(n(170)) : 1.2,
      c: { active: rel >= -20 ? 24 + n(8) : 5, latency: 2.4, bps: 38 * MB },
    }),
    procs: (rel) => [
      proc(11840, "Encoder.exe", rel >= -20 ? 92.4 : 0.1, 640 * MB, 38 * MB),
      proc(4412, "Browser.exe", 3.1, 2.9 * GB, 40 * 1024),
      proc(3876, "MsMpEng.exe", 1.1, 210 * MB, 2 * MB),
    ],
    sensitive: true,
    suspects: [
      suspect({ Cpu: { processes: ["Encoder.exe", "Browser.exe"] } }, "Medium", [
        m(start + 2000, "CpuPct", 99.6, 95),
        { Process: { pid: 11840, name: "Encoder.exe", metric: "CpuPct", value: 92.4 } },
        { Process: { pid: 4412, name: "Browser.exe", metric: "CpuPct", value: 3.1 } },
      ], []),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Browser.exe", value: 2.9 * GB } }],
      [-20_000, { Busiest: { metric: "CpuPct", name: "Encoder.exe", value: 92.4 } }],
      [-20_000, { Busiest: { metric: "IoBytesPerS", name: "Encoder.exe", value: 38 * MB } }],
      [-20_000, { Crossed: { metric: "CpuPct", value: 99.6, threshold: 95, subject: null } }],
      [0, { Stutter: { count: 7, worst_ms: 230 } }],
    ],
  });
}

/** The PC fell with a blue screen five seconds after the last stored second, memory nearly full. */
function blueScreen(): OccurrenceDto {
  const last = at(7, 12, 25);
  const down = last + 5000;
  const boot = at(7, 19, 2);
  const n = noise(8);
  const crash: CrashKind = { BlueScreen: { code: 0x50, params: [0, 0, 0, 0] } };
  const loss = event(boot + 1000, "Microsoft-Windows-Kernel-Power", 41, "PowerLoss", 91002);
  const unexpected = event(boot, "EventLog", 6008, "UnexpectedShutdown", 91001);
  const report = event(boot + 12_000, "Microsoft-Windows-WER-SystemErrorReporting", 1001, "BugCheck", 91010);
  const asEvidence = (e: WinEvent): Evidence => ({
    Event: { ts_ms: e.ts_ms, provider: e.provider, id: e.id, meaning: e.meaning },
  });
  return build({
    id: 8,
    start: last,
    end: down,
    kind: "PostMortem",
    severity: "Severe",
    triggers: [{ Crash: { last_ms: last, down_ms: down, kind: crash } }],
    moment: (rel) =>
      rel > 0
        ? null
        : {
            cpu: 45 + 15 * Math.sin(rel / 7) + n(6),
            mem: 80 + 17 * ((rel + 60) / 60) ** 2,
            c: { active: rel > -30 ? 60 + n(20) : 12 + n(6), latency: rel > -30 ? 40 + n(15) : 2 },
            lag: rel > -10 ? 20 + (rel + 10) * 9 : 0.6,
            faults: rel > -30 ? 1800 : 40,
          },
    procs: () => [
      proc(4412, "Browser.exe", 12, 14.2 * GB, 2 * MB),
      proc(11220, "Game.exe", 31, 9.8 * GB, 6 * MB, 88),
    ],
    events: [unexpected, loss, report],
    truncated: true,
    suspects: [
      suspect({ BlueScreen: { code: 0x50 } }, "High", [loss, unexpected, report].map(asEvidence), ["UpdateDriver"]),
      suspect({ Memory: { process: "Browser.exe" } }, "Medium", [m(last - 20_000, "MemAvailPct", 3, 5)], [
        "CloseMemoryHeavyApp",
      ]),
    ],
    facts: [
      [-60_000, { Busiest: { metric: "PrivateBytes", name: "Browser.exe", value: 14.2 * GB } }],
      [-20_000, { Crossed: { metric: "MemAvailPct", value: 3, threshold: 5, subject: null } }],
      [0, "LastWatched"],
      [5000, { WentDown: { kind: crash } }],
    ],
  });
}

const FIXTURES = new Map(
  [storageFreeze(), appHang(), diskEpisode(), memoryEpisode(), reported(), unclear(), stutters(), blueScreen()].map(
    (o) => [o.id, o],
  ),
);

/** `?covered=1`: a deep capture covers occurrence 6, as the report of an older capture would. */
const COVERED = new URLSearchParams(location.search).get("covered") === "1";

export const handlers: Record<string, Handler> = {
  occurrence: (args) => {
    const found = FIXTURES.get(args.id as number) ?? null;
    if (!found || !COVERED || found.id !== 6) return found;
    return {
      ...found,
      capture: {
        id: 1,
        start_ms: found.start_ms - 60_000,
        end_ms: found.start_ms + 240_000,
        suspects: [
          {
            cause: { DriverModule: { module: "gfxdriver.sys", intermediary: false } },
            confidence: "High",
            evidence: [{ LongRoutine: { ts_ms: found.start_ms, module: "gfxdriver.sys", kind: "Dpc", us: 1_240 } }],
            next_steps: ["UpdateDriver"],
          },
        ],
      },
    };
  },
};
