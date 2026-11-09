import type { IconName } from "../../components/Icon.svelte";
import type { StatusKind } from "../../components/StatusMark.svelte";
import { formatClock, formatDuration, formatPercent, formatTime } from "../../lib/format";
import { t, tn } from "../../lib/i18n.svelte";
import type { Cause, DetailLevel, Episode, OccurrenceSummary } from "../../lib/types";
import { formatMs } from "../occurrence/format";
import { type Day, diskLabel, mostlyOf, type Tone, toneOf, worstOf } from "./day";

const KEY = "ui.today";

/** A short name for a cause: "C: disk", "Chatter.exe", "Display driver". */
export function causeName(cause: Cause): string {
  if (typeof cause === "string") return t(`${KEY}.cause.${cause}`);
  if ("Storage" in cause) {
    const disk = cause.Storage.disk;
    return disk ? t(`${KEY}.cause.Storage`, { disk: diskLabel(disk) }) : t(`${KEY}.cause.Storage.any`);
  }
  if ("DiskIo" in cause) {
    const { disk, process } = cause.DiskIo;
    return process
      ? t(`${KEY}.cause.DiskIo.process`, { disk: diskLabel(disk), process })
      : t(`${KEY}.cause.Storage`, { disk: diskLabel(disk) });
  }
  if ("Memory" in cause) {
    const process = cause.Memory.process;
    return process ? t(`${KEY}.cause.Memory.process`, { process }) : t(`${KEY}.cause.Memory`);
  }
  if ("Cpu" in cause) {
    const process = cause.Cpu.processes[0];
    return process ? t(`${KEY}.cause.Cpu.process`, { process }) : t(`${KEY}.cause.Cpu`);
  }
  if ("Driver" in cause) return t(`${KEY}.cause.Driver`, { core: cause.Driver.core });
  if ("DriverModule" in cause) return cause.DriverModule.module;
  if ("Frames" in cause) return t(`${KEY}.cause.Frames`, { app: cause.Frames.app });
  if ("BlueScreen" in cause) return t(`${KEY}.cause.BlueScreen`);
  if ("PowerCut" in cause) return t(`${KEY}.cause.PowerCut`);
  return cause.App.name;
}

/** "probably because of the C: disk", or `null` when the cause is unclear. */
export function causeReason(cause: Cause): string | null {
  if (cause === "Unclear") return null;
  if (typeof cause === "string") return t(`${KEY}.because.${cause}`);
  if ("Storage" in cause) {
    const disk = cause.Storage.disk;
    return disk ? t(`${KEY}.because.Storage`, { disk: diskLabel(disk) }) : t(`${KEY}.because.Storage.any`);
  }
  if ("DiskIo" in cause) return t(`${KEY}.because.DiskIo`, { disk: diskLabel(cause.DiskIo.disk) });
  if ("Memory" in cause) return t(`${KEY}.because.Memory`);
  if ("Cpu" in cause) return t(`${KEY}.because.Cpu`);
  if ("Driver" in cause) return t(`${KEY}.because.Driver`);
  if ("DriverModule" in cause) return t(`${KEY}.because.DriverModule`, { module: cause.DriverModule.module });
  if ("Frames" in cause) return t(`${KEY}.because.Frames`, { name: cause.Frames.app });
  if ("BlueScreen" in cause) return t(`${KEY}.because.BlueScreen`);
  if ("PowerCut" in cause) return t(`${KEY}.because.PowerCut`);
  return t(`${KEY}.because.App`, { name: cause.App.name });
}

export function causeIcon(cause: Cause): IconName {
  if (cause === "Unresponsive" || cause === "SleepTransition") return "power";
  if (cause === "HardwareFault") return "chip";
  if (typeof cause === "string") return cause === "GpuDriver" ? "gpu" : cause === "Throttling" ? "chip" : "app";
  if ("BlueScreen" in cause || "PowerCut" in cause) return "power";
  if ("Storage" in cause || "DiskIo" in cause) return "disk";
  if ("Memory" in cause) return "memory";
  if ("Cpu" in cause || "Driver" in cause || "DriverModule" in cause) return "chip";
  return "app";
}

const MARK: Record<Tone, { kind: StatusKind; severity: "Severe" | "Medium" | "Light" }> = {
  freeze: { kind: "critical", severity: "Severe" },
  press: { kind: "overload", severity: "Medium" },
  busy: { kind: "attention", severity: "Light" },
};

/** The status symbol of an occurrence in a list, with its gravity as the word: "Grave". */
export function occurrenceMark(o: OccurrenceSummary): { kind: StatusKind; word: string } {
  const mark = MARK[toneOf(o)];
  const word = t(`ui.common.severity.${mark.severity}`);
  return { kind: mark.kind, word: word.charAt(0).toLocaleUpperCase() + word.slice(1) };
}

/** An occurrence's kind, a freeze with its severity: "Severe freeze". */
export function kindName(o: OccurrenceSummary): string {
  return o.kind === "Freeze" ? t(`${KEY}.freeze.${o.severity}`) : t(`ui.common.kind.${o.kind}`);
}

function appName(o: OccurrenceSummary): string | null {
  return typeof o.cause === "object" && "App" in o.cause ? o.cause.App.name : null;
}

/** An occurrence's title at a detail level. */
export function occurrenceTitle(o: OccurrenceSummary, level: DetailLevel): string {
  const ms = o.end_ms - o.start_ms;
  const duration = formatDuration(ms);
  const app = appName(o);
  if (level === "Simple") {
    if (o.kind === "AppHang") {
      return app
        ? t(`${KEY}.title.AppHang`, { name: app, duration })
        : t(`${KEY}.title.AppHang.any`, { duration });
    }
    return t(`${KEY}.title.${o.kind}`, { duration });
  }
  const exact = level === "Technical" ? formatMs(ms) : duration;
  if (o.kind === "Reported") return kindName(o);
  if (o.kind === "PostMortem") return `${kindName(o)} · ${causeName(o.cause)}`;
  if (o.kind === "AppHang" && app) return `${kindName(o)} · ${app} · ${exact}`;
  return `${kindName(o)} · ${exact}`;
}

/** The kind of a pressure episode, as catalog keys name it. */
export function episodeKey(e: Episode): "CpuSaturated" | "MemoryHigh" | "DiskBusy" {
  return typeof e.kind === "string" ? (e.kind as "CpuSaturated" | "MemoryHigh") : "DiskBusy";
}

/** "C:" for a disk episode, empty otherwise. */
export function episodeDisk(e: Episode): string {
  return typeof e.kind === "object" && "DiskBusy" in e.kind ? diskLabel(e.kind.DiskBusy.disk) : "";
}

/** A pressure episode's title at a detail level. */
export function episodeTitle(e: Episode, level: DetailLevel): string {
  const args = {
    duration: formatDuration(e.end_ms - e.start_ms),
    disk: episodeDisk(e),
    peak: formatPercent(e.peak),
  };
  const key = level === "Simple" ? "simple" : level === "Detailed" ? "detailed" : "technical";
  return t(`${KEY}.episode.${episodeKey(e)}.${key}`, args);
}

export function episodeMeta(e: Episode): string {
  const kind = t(`${KEY}.episode.kind`);
  return e.culprit ? `${kind} · ${e.culprit.name}` : kind;
}

export function episodeIcon(e: Episode): IconName {
  const key = episodeKey(e);
  return key === "MemoryHigh" ? "memory" : key === "DiskBusy" ? "disk" : "chip";
}

export interface Part {
  text: string;
  bold: boolean;
}

/** Splits a catalog text around the `{name}` slots still open in it, filled from `bold` in bold. */
export function emphasize(text: string, bold: Record<string, string>): Part[] {
  return text
    .split(/(\{\w+\})/)
    .filter(Boolean)
    .map((piece) => {
      const name = /^\{(\w+)\}$/.exec(piece)?.[1];
      return name && name in bold ? { text: bold[name], bold: true } : { text: piece, bold: false };
    });
}

/** `key.zero` for none, since pt-BR words 0 with the singular. */
function count(key: string, n: number): string {
  return n === 0 ? t(`${key}.zero`) : tn(key, n);
}

/** The summary sentence under the title, at a detail level. */
export function lede(day: Day, level: DetailLevel): Part[] {
  const freezes = day.occurrences.length;
  const pressures = day.episodes.length;
  const mostly = mostlyOf(day.totals);
  const worst = worstOf(day.occurrences);
  if (!mostly && !worst) return emphasize(t(`${KEY}.lede.nothing`), {});
  if (level === "Technical") {
    const counts = {
      occurrences: count(`${KEY}.count.occurrence`, freezes),
      episodes: count(`${KEY}.count.episode`, pressures),
    };
    if (!worst) return emphasize(t(`${KEY}.lede.technical.none`), counts);
    return emphasize(
      t(`${KEY}.lede.technical`, {
        clock: formatClock(worst.start_ms),
        kind: kindName(worst),
        ms: formatMs(worst.end_ms - worst.start_ms),
        cause: causeName(worst.cause),
      }),
      counts,
    );
  }
  const opening = mostly ? t(`${KEY}.lede.mostly.${mostly}`) : t(`${KEY}.lede.mostly.unwatched`);
  const withPressure = level === "Detailed" && pressures > 0;
  const bold = {
    count: freezes ? tn(`${KEY}.count.freeze`, freezes) : "",
    episodes: tn(`${KEY}.count.pressure`, pressures),
  };
  if (!worst) {
    return emphasize(t(`${KEY}.lede.none${withPressure ? ".pressure" : ""}`, { mostly: opening }), bold);
  }
  const first = t(`${KEY}.lede.some${withPressure ? ".pressure" : ""}`, { mostly: opening });
  const reason = causeReason(worst.cause);
  if (worst.kind === "PostMortem") {
    const fell = t(`${KEY}.lede.worst.PostMortem${reason ? "" : ".unclear"}`, {
      time: formatTime(worst.end_ms),
      reason: reason ?? "",
    });
    return emphasize(`${first} ${fell}`, bold);
  }
  const second = t(`${KEY}.lede.worst${level === "Detailed" ? ".detailed" : ""}${reason ? "" : ".unclear"}`, {
    time: formatTime(worst.start_ms),
    duration: formatDuration(worst.end_ms - worst.start_ms),
    kind: kindName(worst),
    reason: reason ?? "",
  });
  return emphasize(`${first} ${second}`, bold);
}
