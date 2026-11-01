import { formatNumber } from "../../lib/format";
import { lang, t } from "../../lib/i18n.svelte";

// Sizes, dates and names the report needs on top of lib/format.

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** "512 MB", "9,1 GB": multiples of 1024, one decimal below 10. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${formatNumber(value, unit > 0 && value < 10 ? 1 : 0)} ${UNITS[unit]}`;
}

export function formatRate(bytesPerS: number): string {
  return `${formatBytes(bytesPerS)}/s`;
}

export function formatMs(ms: number): string {
  return `${formatNumber(Math.round(ms))} ms`;
}

/** "domingo, 1 de novembro" in pt-BR, "Sunday, November 1" in English. */
export function formatDay(ms: number): string {
  return new Intl.DateTimeFormat(lang(), { weekday: "long", day: "numeric", month: "long" }).format(ms);
}

/** "1 de nov." in pt-BR, "Nov 1" in English. */
export function formatDayShort(ms: number): string {
  return new Intl.DateTimeFormat(lang(), { day: "numeric", month: "short" }).format(ms);
}

/** The full date and time, year included, for a report read outside the app. */
export function formatStamp(ms: number): string {
  return new Intl.DateTimeFormat(lang(), { dateStyle: "full", timeStyle: "medium" }).format(ms);
}

export function sameDay(a: number, b: number): boolean {
  return new Date(a).toDateString() === new Date(b).toDateString();
}

/** "A, B e C" in pt-BR, "A, B, and C" in English. */
export function formatList(items: string[]): string {
  return new Intl.ListFormat(lang(), { type: "conjunction" }).format(items);
}

/** "−12 s", "0 s", "+5 s": seconds from the start of the occurrence. */
export function formatOffset(seconds: number): string {
  const s = Math.round(seconds);
  return s === 0 ? "0 s" : `${s > 0 ? "+" : "−"}${Math.abs(s)} s`;
}

/** A PhysicalDisk instance as people know it: "0 C:" is "C:", a disk with no letter is "disk 0". */
export function diskName(instance: string): string {
  const match = /^(\d+)\s*(.*)$/.exec(instance.trim());
  if (!match) return instance;
  return match[2] || t("ui.occurrence.disk_n", { n: match[1] });
}

/** "Discord.exe" is "Discord" and "steam.exe" is "Steam". */
export function appName(exe: string): string {
  const name = exe.replace(/\.exe$/i, "");
  return name.charAt(0).toLocaleUpperCase() + name.slice(1);
}
