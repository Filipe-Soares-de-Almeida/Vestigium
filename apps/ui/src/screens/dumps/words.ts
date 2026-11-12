import type { Freed } from "../../lib/dumps.svelte";
import { t, tn } from "../../lib/i18n.svelte";
import type {
  BugCheckDto,
  Confidence,
  CrashControlDto,
  DetailLevel,
  DumpCulprit,
  DumpDto,
  DumpsDto,
} from "../../lib/types";
import { formatTime } from "../../lib/format";
import { appName, formatBytes, formatDayShort, sameDay } from "../occurrence/format";

// The words of the dumps page, built from typed dumps and Windows' blue screen reports at the reader's level.

type Args = Record<string, string | number>;

const k = (key: string, args?: Args) => t(`ui.dumps.${key}`, args);
const kn = (key: string, count: number, args?: Args) => tn(`ui.dumps.${key}`, count, args);

/** Dumps of one fall are this close; a report comes with the next boot, so it may come later. */
const FALL_REACH_MS = 5 * 60_000;
const REPORT_REACH_MS = 30 * 60_000;

export type Group = "blue" | "live" | "app";
export const GROUPS: Group[] = ["blue", "live", "app"];

/** One line of the page: a stored dump, or a blue screen Windows reported whose dump is not listed. */
export interface Entry {
  key: string;
  dump: DumpDto | null;
  report: BugCheckDto | null;
  group: Group;
  at: number;
  /** A minidump of a fall whose complete dump is also on disk. */
  twin: boolean;
}

const GROUP_OF: Record<DumpDto["kind"], Group> = {
  KernelMemory: "blue",
  KernelMini: "blue",
  LiveKernel: "live",
  App: "app",
};

export const TONE: Record<Group, string> = { blue: "freeze", live: "press", app: "busy" };
export const ICON: Record<Group, "power" | "live" | "app"> = { blue: "power", live: "live", app: "app" };

const samePath = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
const isKernel = (d: DumpDto) => d.kind === "KernelMemory" || d.kind === "KernelMini";

/** Every dump and every report without a listed dump, newest first, each report paired with its dump. */
export function entries(dto: DumpsDto): Entry[] {
  const used = new Set<BugCheckDto>();
  const reportOf = (d: DumpDto): BugCheckDto | null => {
    const byPath = dto.reports.find((r) => r.dump_path && samePath(r.dump_path, d.path));
    if (byPath) return byPath;
    if (!isKernel(d)) return null;
    const near = dto.reports.filter((r) => Math.abs(r.ts_ms - d.crash_ms) <= REPORT_REACH_MS);
    return near.sort((a, b) => Math.abs(a.ts_ms - d.crash_ms) - Math.abs(b.ts_ms - d.crash_ms))[0] ?? null;
  };
  const list: Entry[] = dto.dumps.map((d) => {
    const report = reportOf(d);
    if (report) used.add(report);
    const twin =
      d.kind === "KernelMini" &&
      dto.dumps.some(
        (m) => m.kind === "KernelMemory" && m.deleted_ms === null && Math.abs(m.crash_ms - d.crash_ms) <= FALL_REACH_MS,
      );
    return { key: `d${d.id}`, dump: d, report, group: GROUP_OF[d.kind], at: d.crash_ms, twin };
  });
  for (const r of dto.reports) {
    if (used.has(r) || dto.dumps.some((d) => r.dump_path && samePath(r.dump_path, d.path))) continue;
    list.push({ key: `r${r.ts_ms}`, dump: null, report: r, group: "blue", at: r.ts_ms, twin: false });
  }
  return list.sort((a, b) => b.at - a.at);
}

export const present = (e: Entry) => e.dump === null || e.dump.deleted_ms === null;
export const isRead = (e: Entry) => e.dump?.analyzed_ms != null && e.dump.fault !== null;

/** The stored dump of the fall at `downMs` with this stop code: the nearest, the minidump on a tie. */
export function dumpOfFall(dto: DumpsDto, downMs: number, code: number | null): DumpDto | null {
  const fits = dto.dumps
    .filter((d) => isKernel(d) && Math.abs(d.crash_ms - downMs) <= FALL_REACH_MS)
    .filter((d) => code === null || !d.fault || ("BugCheck" in d.fault && d.fault.BugCheck.code === code));
  const rank = (d: DumpDto) => Math.abs(d.crash_ms - downMs) * 2 + (d.kind === "KernelMini" ? 0 : 1);
  return fits.sort((a, b) => rank(a) - rank(b))[0] ?? null;
}

// ---- Codes ----

/** "0x00000050", as Windows writes a code. */
export function hex(code: number): string {
  return `0x${(code >>> 0).toString(16).toUpperCase().padStart(8, "0")}`;
}

/** Windows' names for the stop codes the page explains. */
const STOP_NAMES: Record<number, string> = {
  0x0a: "IRQL_NOT_LESS_OR_EQUAL",
  0x19: "BAD_POOL_HEADER",
  0x1a: "MEMORY_MANAGEMENT",
  0x1e: "KMODE_EXCEPTION_NOT_HANDLED",
  0x24: "NTFS_FILE_SYSTEM",
  0x3b: "SYSTEM_SERVICE_EXCEPTION",
  0x4e: "PFN_LIST_CORRUPT",
  0x50: "PAGE_FAULT_IN_NONPAGED_AREA",
  0x77: "KERNEL_STACK_INPAGE_ERROR",
  0x7a: "KERNEL_DATA_INPAGE_ERROR",
  0x7e: "SYSTEM_THREAD_EXCEPTION_NOT_HANDLED",
  0x7f: "UNEXPECTED_KERNEL_MODE_TRAP",
  0x8e: "KERNEL_MODE_EXCEPTION_NOT_HANDLED",
  0x9c: "MACHINE_CHECK_EXCEPTION",
  0x9f: "DRIVER_POWER_STATE_FAILURE",
  0xbe: "ATTEMPTED_WRITE_TO_READONLY_MEMORY",
  0xc2: "BAD_POOL_CALLER",
  0xc4: "DRIVER_VERIFIER_DETECTED_VIOLATION",
  0xd1: "DRIVER_IRQL_NOT_LESS_OR_EQUAL",
  0xea: "THREAD_STUCK_IN_DEVICE_DRIVER",
  0xef: "CRITICAL_PROCESS_DIED",
  0xf4: "CRITICAL_OBJECT_TERMINATION",
  0xfc: "ATTEMPTED_EXECUTE_OF_NOEXECUTE_MEMORY",
  0x101: "CLOCK_WATCHDOG_TIMEOUT",
  0x116: "VIDEO_TDR_FAILURE",
  0x117: "VIDEO_TDR_TIMEOUT_DETECTED",
  0x124: "WHEA_UNCORRECTABLE_ERROR",
  0x133: "DPC_WATCHDOG_VIOLATION",
  0x139: "KERNEL_SECURITY_CHECK_FAILURE",
  0x141: "VIDEO_ENGINE_TIMEOUT_DETECTED",
  0x154: "UNEXPECTED_STORE_EXCEPTION",
};

/** Windows' names for the exception codes the page explains. */
const EXCEPTION_NAMES: Record<number, string> = {
  0x80000003: "BREAKPOINT",
  0xc0000005: "ACCESS_VIOLATION",
  0xc0000006: "IN_PAGE_ERROR",
  0xc000001d: "ILLEGAL_INSTRUCTION",
  0xc0000094: "INTEGER_DIVIDE_BY_ZERO",
  0xc00000fd: "STACK_OVERFLOW",
  0xc0000374: "HEAP_CORRUPTION",
  0xc0000409: "STACK_BUFFER_OVERRUN",
  0xc0000602: "FAIL_FAST_EXCEPTION",
  0xe0434352: "CLR_EXCEPTION",
  0xe06d7363: "CPP_EH_EXCEPTION",
};

const VIDEO_STOPS = [0x116, 0x117, 0x141, 0xea];

/** 0x1000007E repeats 0x7E with other parameters. */
const stopCode = (code: number) => (code >>> 0 >= 0x10000000 && STOP_NAMES[code & 0xfffffff] ? code & 0xfffffff : code >>> 0);

export function stopName(code: number): string {
  return STOP_NAMES[stopCode(code)] ?? "";
}

/** A clause such as "a driver held…" as a sentence of its own. */
const capital = (s: string) => s.charAt(0).toLocaleUpperCase() + s.slice(1);

/** What the stop code means, as a clause that follows "Blue screen:". */
export function stopSentence(code: number): string {
  const c = stopCode(code);
  return STOP_NAMES[c] ? t(`ui.dumps.stop.${c.toString(16)}`) : k("stop.other", { code: hex(code) });
}

function exceptionSentence(code: number): string {
  const c = code >>> 0;
  return EXCEPTION_NAMES[c] ? t(`ui.dumps.exception.${c.toString(16)}`) : k("exception.other", { code: hex(c) });
}

/** The stop code Windows gave for an entry: from the dump once read, else from its report. */
function codeOf(e: Entry): number | null {
  const fault = e.dump?.fault;
  if (fault && "BugCheck" in fault) return fault.BugCheck.code;
  return e.report?.code ?? null;
}

// ---- Who it points at ----

type Family = "video" | "network" | "storage" | "audio" | "usb" | "security" | "files" | "windows";

const FAMILIES: [Family, RegExp][] = [
  ["video", /^(nvlddmkm|nvkflt|atikmdag|atikmpag|amdkmdag|amdkmpfd|igdkmdn?d?(32|64)|dxgkrnl|dxgmms\d|basicdisplay|basicrender)\.sys$/],
  ["network", /^(tcpip|ndis|netio|netwtw\w*|netwbw\w*|netadaptercx|rt640x64|rtwlane\w*|e1\w+x64|e2xw\w*|athw\w*|bcmwl\w*|vwififlt|wfplwfs)\.sys$/],
  ["storage", /^(storport|stornvme|storahci|ataport|iastor\w*|disk|classpnp|partmgr|volmgr|volsnap)\.sys$/],
  ["audio", /^(portcls|hdaudbus|hdaudio|rtkvhd64|ks)\.sys$/],
  ["usb", /^(usbxhci|usbhub3|ucx01000|usbport|usbhub|usbccgp)\.sys$/],
  ["security", /^(wdfilter|wdboot|wdnisdrv)\.sys$/],
  ["files", /^(ntfs|refs|fastfat|exfat|fltmgr|cldflt|wof)\.sys$/],
  ["windows", /^(win32k|win32kbase|win32kfull)\.sys$/],
];

/** What kind of driver a module is, from its name or, for a display driver, from the graphics kernel beside it. */
function familyOf(module: string, d: DumpDto): Family | null {
  const name = module.toLowerCase();
  const known = FAMILIES.find(([, re]) => re.test(name));
  if (known) return known[0];
  if (!name.endsWith(".sys")) return null;
  const stack = d.stack_modules.map((m) => m.toLowerCase());
  if (stack.some((m) => /^(dxgkrnl|dxgmms\d)\.sys$/.test(m))) return "video";
  const fault = d.fault;
  return fault && "BugCheck" in fault && VIDEO_STOPS.includes(stopCode(fault.BugCheck.code)) ? "video" : null;
}

export interface CulpritView {
  name: string;
  /** "high confidence · the fault address falls inside it" */
  sub: string;
  confidence: string;
  icon: "chip" | "app" | "memory";
}

const confidence = (c: Confidence) => t(`ui.occurrence.confidence.${c}`);

export function culpritView(d: DumpDto, c: DumpCulprit, level: DetailLevel): CulpritView {
  if (c === "Hardware") {
    return { name: k("culprit.hardware"), sub: k("culprit.hardware.why"), confidence: confidence("High"), icon: "memory" };
  }
  if ("App" in c) {
    const app = appName(c.App.name);
    const name = level === "Technical" ? c.App.name : k("culprit.app_itself", { app });
    return { name, sub: `${confidence("Low")} · ${k("culprit.why.app")}`, confidence: confidence("Low"), icon: "app" };
  }
  const { name: module, confidence: conf, intermediary } = c.Module;
  const app = d.kind === "App";
  const family = app ? null : familyOf(module, d);
  const plain = app
    ? k("culprit.app_module", { module, app: appName(d.process ?? "") })
    : family
      ? k(`family.${family}`)
      : k("culprit.driver", { module });
  const name =
    level === "Technical" ? module : level === "Detailed" && family ? `${plain} (${module})` : plain;
  const why = intermediary
    ? k("culprit.why.intermediary")
    : conf === "High"
      ? k(app ? "culprit.why.inside_app" : "culprit.why.inside")
      : k(app ? "culprit.why.stack_app" : "culprit.why.stack");
  return { name, sub: `${confidence(conf)} · ${why}`, confidence: confidence(conf), icon: app ? "app" : "chip" };
}

/** The culprit as one short name, the same for every dump it appears in. */
export function culpritKey(d: DumpDto): string | null {
  const c = d.culprit;
  if (!c) return null;
  if (c === "Hardware") return "hardware";
  return "App" in c ? `app:${c.App.name.toLowerCase()}` : `module:${c.Module.name.toLowerCase()}`;
}

// ---- Lines ----

const appOfFile = (path: string) => /([^\\/]+?\.exe)\.\d+\.dmp$/i.exec(path)?.[1] ?? null;
export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function title(e: Entry, level: DetailLevel): string {
  const code = codeOf(e);
  const tech = level === "Technical";
  const d = e.dump;
  if (!d) {
    const r = e.report!;
    if (tech) return k(r.dump_path ? "title.tech.unread" : "title.tech.no_dump", { code: hex(r.code) });
    return k(r.dump_path ? "title.report" : "title.report.no_dump", { code: hex(r.code) });
  }
  const fault = isRead(e) ? d.fault : null;
  if (d.kind === "App") {
    const exe = d.process ?? appOfFile(d.path) ?? "";
    if (fault && "Exception" in fault) {
      const c = fault.Exception.code;
      if (tech) return [hex(c), EXCEPTION_NAMES[c >>> 0], k("title.tech.in", { exe })].filter(Boolean).join(" ");
      return k(exe ? "title.app" : "title.app.someone", { app: appName(exe), what: exceptionSentence(c) });
    }
    if (tech) return k("title.tech.unread", { code: exe || fileName(d.path) });
    return exe ? k("title.app.unread", { app: appName(exe) }) : k("title.app.unread.someone");
  }
  if (fault && "BugCheck" in fault) {
    const c = fault.BugCheck.code;
    if (tech) return [hex(c), stopName(c)].filter(Boolean).join(" ");
    if (e.twin) return k("title.twin");
    return d.kind === "LiveKernel" ? capital(stopSentence(c)) : k("title.blue", { what: stopSentence(c) });
  }
  if (tech) return k("title.tech.unread", { code: code !== null ? hex(code) : fileName(d.path) });
  if (d.kind === "LiveKernel") return k("title.live.unread");
  return code !== null ? k("title.blue.unread", { code: hex(code) }) : k("title.blue.unread.no_code");
}

export function sub(e: Entry, level: DetailLevel): string {
  const d = e.dump;
  if (!d) return e.report!.dump_path ? k("sub.unlisted") : k("sub.no_dump");
  if (d.deleted_ms !== null) return k("sub.gone");
  if (d.analyzed_ms !== null && !d.fault) return k("sub.unreadable");
  if (!isRead(e)) return k(level === "Technical" ? "sub.unread.tech" : "sub.unread");
  if (!d.culprit) return k("culprit.none");
  const c = culpritView(d, d.culprit, level);
  return `${c.name} · ${c.confidence}`;
}

/** "today, 21:52" or "Oct 28, 22:10". */
export function at(ms: number): string {
  return sameDay(ms, Date.now())
    ? k("today_at", { time: formatTime(ms) })
    : k("day_at", { day: formatDayShort(ms), time: formatTime(ms) });
}

export const KIND_LABEL = (d: DumpDto, level: DetailLevel) =>
  k(`kind.${d.kind}${level === "Technical" ? ".tech" : ""}`);

// ---- What the dump says ----

export interface Said {
  main: string;
  sub: string | null;
  mono: boolean;
}

export function said(d: DumpDto, level: DetailLevel): Said[] {
  const out: Said[] = [];
  const fault = d.fault;
  if (fault && "BugCheck" in fault) {
    const { code, params } = fault.BugCheck;
    const name = stopName(code);
    out.push({
      main: level === "Technical" && name ? `${hex(code)} ${name}` : k("said.stop", { code: hex(code) }),
      sub: level === "Simple" ? null : params.join(", "),
      mono: true,
    });
  } else if (fault && "Exception" in fault) {
    const { code, address } = fault.Exception;
    const name = EXCEPTION_NAMES[code >>> 0];
    out.push({
      main: level === "Technical" && name ? `${hex(code)} ${name}` : k("said.exception", { code: hex(code) }),
      sub: level === "Simple" ? null : k("said.address", { address }),
      mono: true,
    });
  }
  if (d.stack_modules.length) {
    out.push({
      main: k(level === "Technical" ? "said.stack.tech" : "said.stack"),
      sub: d.stack_modules.join(" › "),
      mono: false,
    });
  }
  if (level !== "Simple" && d.process) out.push({ main: k("said.process", { exe: d.process }), sub: null, mono: false });
  if (level === "Technical" && d.module_count !== null) {
    out.push({ main: kn("said.modules", d.module_count), sub: k("said.engine"), mono: false });
  }
  return out;
}

// ---- What to do ----

export interface Step {
  key: string;
  text: string;
}

export function steps(d: DumpDto): Step[] {
  const step = (key: string, args?: Args) => ({ key, text: k(`step.${key}`, args) });
  const c = d.culprit;
  if (c === "Hardware") return [step("memtest"), step("temps"), step("overclock")];
  if (d.kind === "App") {
    const app = appName(d.process ?? appOfFile(d.path) ?? "");
    if (c && "Module" in c) return [step("app_verify", { app }), step("app_update", { app })];
    return [step("app_update", { app })];
  }
  if (c && "Module" in c) {
    const family = familyOf(c.Module.name, d);
    if (family === "video") return [step("video_update"), step("overclock_gpu"), step("video_clean")];
    if (family === "windows" || family === "files") return [step("windows_update"), step("sfc"), step("memtest_again")];
    // A Windows component that serves other drivers has no update of its own: the device behind it does.
    if (c.Module.intermediary) {
      return [step(family === "storage" ? "storage_device" : "chipset"), step("undo_recent"), step("memtest_again")];
    }
    return [step("driver_update", { module: c.Module.name }), step("undo_recent"), step("memtest_again")];
  }
  return [step("memtest_again"), step("chipset")];
}

// ---- The page ----

/** A sentence in parts, some of them bold. */
export interface Part {
  text: string;
  bold: boolean;
}

/** One sentence on what the dumps say, or why there is nothing to say yet. */
export function lede(list: Entry[], dto: DumpsDto, level: DetailLevel): Part[] {
  const shown = list.filter(present);
  const dumps = shown.flatMap((e) => (e.dump ? [e.dump] : []));
  const size = formatBytes(dto.space_bytes);
  const unread = shown.filter((e) => !isRead(e) && (e.dump !== null || e.report?.dump_path)).length;
  if (!shown.length) {
    if (list.length) return [{ text: k("lede.all_gone"), bold: false }];
    return [{ text: k(dto.crash?.dump_type === 0 ? "lede.none.off" : "lede.none"), bold: false }];
  }
  if (!dumps.length) {
    return [
      { text: kn("lede.reports.head", shown.length), bold: true },
      { text: ` ${k(unread ? "lede.reports.unlisted" : "lede.reports.no_dump")}`, bold: false },
    ];
  }
  const tail = unread && unread < shown.length ? [{ text: ` ${kn("lede.some_unread", unread)}`, bold: false }] : [];
  if (!shown.some(isRead)) {
    return [
      { text: kn("lede.unread.head", dumps.length, { size }), bold: true },
      { text: ` ${k("lede.unread.tail")}`, bold: false },
    ];
  }
  const counts = new Map<string, DumpDto[]>();
  for (const d of dumps) {
    const key = culpritKey(d);
    if (key) counts.set(key, [...(counts.get(key) ?? []), d]);
  }
  const [top] = [...counts.values()].sort((a, b) => b.length - a.length);
  if (level === "Technical") {
    const blue = shown.filter((e) => e.group === "blue").length;
    const most = top ? k("lede.tech.most", { name: culpritView(top[0], top[0].culprit!, "Technical").name, count: top.length }) : "";
    return [{ text: kn("lede.tech", shown.length, { blue, size }) + most, bold: false }, ...tail];
  }
  if (top && top.length > 1) {
    const name = culpritView(top[0], top[0].culprit!, "Simple").name;
    return [
      { text: k("lede.repeat.head", { name, count: top.length, total: dumps.length }), bold: true },
      { text: ` ${k("lede.size", { size })}`, bold: false },
      ...tail,
    ];
  }
  const last = shown.find((e) => e.group === "blue" && isRead(e));
  if (last) {
    return [
      { text: k("lede.last.head", { when: at(last.at), what: stopSentence(codeOf(last) ?? 0) }), bold: true },
      { text: ` ${k("lede.size", { size })}`, bold: false },
      ...tail,
    ];
  }
  return [
    { text: kn("lede.plain.head", dumps.length, { size }), bold: true },
    ...tail,
  ];
}

/** Windows' blue screen setting as a sentence, and whether it leaves something to read. */
export function setting(crash: CrashControlDto | null, level: DetailLevel): { ok: boolean; text: string } {
  if (!crash) return { ok: false, text: k("setting.unknown") };
  if (level === "Technical") {
    return {
      ok: crash.dump_type !== 0,
      text: k("setting.tech", {
        type: crash.dump_type,
        file: crash.dump_file,
        dir: crash.minidump_dir,
        overwrite: crash.overwrite ? 1 : 0,
      }),
    };
  }
  const known = [0, 1, 2, 3, 7].includes(crash.dump_type) ? crash.dump_type : "other";
  return { ok: crash.dump_type !== 0, text: k(`setting.${known}`) };
}

/** The space each group takes on disk. */
export function space(list: Entry[]): { group: Group; bytes: number }[] {
  return GROUPS.map((group) => ({
    group,
    bytes: list
      .filter((e) => e.group === group && e.dump && e.dump.deleted_ms === null)
      .reduce((sum, e) => sum + (e.dump?.size_bytes ?? 0), 0),
  })).filter((s) => s.bytes > 0);
}

/** When the dumps were last read, or null when none was. */
export function lastRead(dto: DumpsDto): number | null {
  const times = dto.dumps.flatMap((d) => (d.analyzed_ms !== null ? [d.analyzed_ms] : []));
  return times.length ? Math.max(...times) : null;
}

/** What a delete that did not remove every dump asked for says, and why when the helper told. */
export function partialDeleteText(freed: Freed): string {
  const args = { count: freed.count, asked: freed.asked, size: formatBytes(freed.bytes) };
  if (freed.refused + freed.failed === 0) return k("delete.toast.partial", args);
  return k("delete.toast.partial_reasons", { ...args, refused: freed.refused, failed: freed.failed });
}
