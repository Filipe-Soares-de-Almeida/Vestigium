// Dumps page data. URL: ?dumps=none|unread|full (default full) &dump-ms=<wait> &dump-fail=1
// The blue screen of today at 7:12 is the fall of ?fall=1.
// Every name, path and code here is invented; nothing was read from a real PC.
import type { DeleteFinished, DumpDto, DumpScanFinished, DumpsDto } from "../../lib/types";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const MB = 1024 * 1024;
const now = Date.now();
const midnight = new Date();
midnight.setHours(0, 0, 0, 0);
const fall = midnight.getTime() + (7 * 60 + 12) * 60_000 + 30_000;

const kernelMemory: DumpDto = {
  id: 1,
  path: "C:\\Windows\\MEMORY.DMP",
  kind: "KernelMemory",
  crash_ms: fall,
  file_ms: fall + 6 * 60_000,
  size_bytes: 1_980 * MB,
  found_ms: fall + 7 * 60_000,
  analyzed_ms: now - 10 * 60_000,
  deleted_ms: null,
  fault: {
    BugCheck: {
      code: 0x50,
      params: ["0xffffc00012340000", "0x0000000000000000", "0xfffff80123456789", "0x0000000000000002"],
    },
  },
  fault_module: "gfxdriver.sys",
  stack_modules: ["gfxdriver.sys", "dxgkrnl.sys", "ntoskrnl.exe"],
  process: null,
  module_count: 212,
  culprit: { Module: { name: "gfxdriver.sys", confidence: "High", intermediary: false } },
};

const kernelMini: DumpDto = {
  ...kernelMemory,
  id: 2,
  path: "C:\\Windows\\Minidump\\110126-10000-01.dmp",
  kind: "KernelMini",
  size_bytes: 2 * MB,
};

const olderMini: DumpDto = {
  id: 3,
  path: "C:\\Windows\\Minidump\\101826-9000-01.dmp",
  kind: "KernelMini",
  crash_ms: now - 15 * DAY,
  file_ms: now - 15 * DAY + 4 * 60_000,
  size_bytes: 2 * MB,
  found_ms: now - 15 * DAY,
  analyzed_ms: now - 2 * DAY,
  deleted_ms: null,
  fault: {
    BugCheck: {
      code: 0x0a,
      params: ["0xffffa00011112222", "0x0000000000000002", "0x0000000000000000", "0xfffff80011223344"],
    },
  },
  fault_module: "ntoskrnl.exe",
  stack_modules: ["ntoskrnl.exe"],
  process: null,
  module_count: 205,
  culprit: null,
};

const liveReport: DumpDto = {
  id: 4,
  path: "C:\\Windows\\LiveKernelReports\\WATCHDOG\\WATCHDOG-20261028-2210.dmp",
  kind: "LiveKernel",
  crash_ms: now - 5 * DAY,
  file_ms: now - 5 * DAY,
  size_bytes: 412 * MB,
  found_ms: now - 2 * DAY,
  analyzed_ms: now - 2 * DAY,
  deleted_ms: null,
  fault: {
    BugCheck: {
      code: 0x141,
      params: ["0xffffc00055556666", "0xfffff80077778888", "0x0000000000000000", "0x0000000000000000"],
    },
  },
  fault_module: "gfxdriver.sys",
  stack_modules: ["gfxdriver.sys", "dxgkrnl.sys"],
  process: null,
  module_count: 198,
  culprit: { Module: { name: "gfxdriver.sys", confidence: "High", intermediary: false } },
};

const gameCrash: DumpDto = {
  id: 5,
  path: "C:\\Users\\Someone\\AppData\\Local\\CrashDumps\\Game.exe.4120.dmp",
  kind: "App",
  crash_ms: now - 3 * DAY,
  file_ms: now - 3 * DAY,
  size_bytes: 74 * MB,
  found_ms: now - 3 * DAY,
  analyzed_ms: now - 2 * DAY,
  deleted_ms: null,
  fault: { Exception: { code: 0xc0000005, address: "0x00007ffd545e5d56" } },
  fault_module: "Engine.dll",
  stack_modules: ["ntdll.dll", "KERNELBASE.dll", "Engine.dll", "Scripting.dll"],
  process: "Game.exe",
  module_count: 157,
  culprit: { Module: { name: "Engine.dll", confidence: "High", intermediary: false } },
};

const toolCrash: DumpDto = {
  id: 6,
  path: "C:\\Users\\Someone\\AppData\\Local\\CrashDumps\\Builder.exe.7788.dmp",
  kind: "App",
  crash_ms: now - 9 * DAY,
  file_ms: now - 9 * DAY,
  size_bytes: 2 * MB,
  found_ms: now - 9 * DAY,
  analyzed_ms: null,
  deleted_ms: null,
  fault: null,
  fault_module: null,
  stack_modules: [],
  process: null,
  module_count: null,
  culprit: null,
};

const deleted: DumpDto = {
  ...olderMini,
  id: 7,
  path: "C:\\Windows\\Minidump\\092026-8000-01.dmp",
  crash_ms: now - 40 * DAY,
  file_ms: now - 40 * DAY,
  deleted_ms: now - 20 * DAY,
};

/** The falls of occurrences 13 and 14 on the report page: a hardware stop and a storage layer. */
const hardwareFall = midnight.getTime() + (5 * 60 + 12) * 60_000 + 30_000;
const hardwareStop: DumpDto = {
  ...kernelMini,
  id: 8,
  path: "C:\\Windows\\Minidump\\110126-11000-01.dmp",
  crash_ms: hardwareFall,
  file_ms: hardwareFall + 5 * 60_000,
  fault: {
    BugCheck: {
      code: 0x124,
      params: ["0x0000000000000000", "0xffffa00012345678", "0x00000000be000000", "0x0000000000000800"],
    },
  },
  fault_module: null,
  stack_modules: ["ntoskrnl.exe", "hal.dll"],
  culprit: "Hardware",
};

const storageFall = midnight.getTime() + (4 * 60 + 12) * 60_000 + 30_000;
const storageLayer: DumpDto = {
  ...kernelMini,
  id: 9,
  path: "C:\\Windows\\Minidump\\110126-12000-01.dmp",
  crash_ms: storageFall,
  file_ms: storageFall + 5 * 60_000,
  fault: {
    BugCheck: {
      code: 0xa,
      params: ["0xffffc00012340000", "0x0000000000000002", "0x0000000000000000", "0xfffff80123456789"],
    },
  },
  fault_module: "storport.sys",
  stack_modules: ["storport.sys", "ntoskrnl.exe"],
  culprit: { Module: { name: "storport.sys", confidence: "Low", intermediary: true } },
};

let list: DumpDto[] = [];

function initial(): DumpDto[] {
  const level = query.get("dumps") ?? "full";
  if (level === "none") return [];
  const all = [kernelMemory, kernelMini, liveReport, gameCrash, toolCrash, olderMini, deleted, hardwareStop, storageLayer];
  if (level === "unread") {
    return all.map((d) => ({
      ...d,
      analyzed_ms: null,
      fault: null,
      fault_module: null,
      stack_modules: [],
      process: null,
      module_count: null,
      culprit: null,
    }));
  }
  return all;
}

list = initial();

function data(): DumpsDto {
  return {
    dumps: list,
    space_bytes: list.filter((d) => d.deleted_ms === null).reduce((sum, d) => sum + d.size_bytes, 0),
    reports:
      query.get("dumps") === "none"
        ? []
        : [
            {
              ts_ms: kernelMemory.crash_ms + 7 * 60_000,
              code: 0x50,
              params: kernelMemory.fault && "BugCheck" in kernelMemory.fault ? kernelMemory.fault.BugCheck.params : [],
              dump_path: kernelMemory.path,
            },
          ],
    crash: { dump_type: 7, dump_file: "C:\\Windows\\MEMORY.DMP", minidump_dir: "C:\\Windows\\Minidump", overwrite: true },
  };
}

const wait = () => new Promise((resolve) => setTimeout(resolve, Number(query.get("dump-ms") ?? 1500)));

export const handlers: Record<string, Handler> = {
  dumps: () => data(),
  start_dump_scan: async () => {
    await wait();
    if (query.get("dump-fail") === "1") return { dumps: [], exit_code: 1 } satisfies DumpScanFinished;
    const full = [kernelMemory, kernelMini, liveReport, gameCrash, toolCrash, olderMini, deleted, hardwareStop, storageLayer];
    list = list.map((d) => full.find((f) => f.id === d.id && f.analyzed_ms !== null) ?? d);
    return { dumps: list.map((d) => d.id), exit_code: 0 } satisfies DumpScanFinished;
  },
  delete_dumps: async (args) => {
    await wait();
    const ids = args.ids as number[];
    const at = Date.now();
    list = list.map((d) => (ids.includes(d.id) && d.deleted_ms === null ? { ...d, deleted_ms: at } : d));
    return { deleted: ids.length, exit_code: 0 } satisfies DeleteFinished;
  },
};
