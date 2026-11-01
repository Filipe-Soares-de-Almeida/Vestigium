import { lang } from "./i18n.svelte";

// Numbers, times and durations in the window's language.

export function formatNumber(value: number, digits = 0): string {
  return new Intl.NumberFormat(lang(), {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value);
}

/** "0,1%" in pt-BR, "0.1%" in English. */
export function formatPercent(value: number, digits = 0): string {
  return new Intl.NumberFormat(lang(), {
    style: "percent",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value / 100);
}

/** Hour and minute of a Unix time in milliseconds. */
export function formatTime(ms: number): string {
  return new Intl.DateTimeFormat(lang(), { hour: "numeric", minute: "2-digit" }).format(ms);
}

/** Hour, minute and second of a Unix time in milliseconds. */
export function formatClock(ms: number): string {
  return new Intl.DateTimeFormat(lang(), {
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  }).format(ms);
}

/** "4,2 s", "12 s", "2 min 5 s", "1 h 05". */
export function formatDuration(ms: number): string {
  const s = ms / 1000;
  if (s < 10 && Math.round(s * 10) % 10 !== 0) return `${formatNumber(s, 1)} s`;
  const total = Math.round(s);
  if (total < 60) return `${total} s`;
  const minutes = Math.floor(total / 60);
  if (minutes < 60) {
    const rest = total % 60;
    return rest ? `${minutes} min ${rest} s` : `${minutes} min`;
  }
  return `${Math.floor(minutes / 60)} h ${String(minutes % 60).padStart(2, "0")}`;
}
