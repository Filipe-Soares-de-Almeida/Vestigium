import { invoke } from "@tauri-apps/api/core";

import { app } from "../../lib/app.svelte";
import { lang } from "../../lib/i18n.svelte";
import type { Retention, Settings } from "../../lib/types";

// What the Settings screen reads and writes besides `lib/api.ts`; calls reject with a `UiError`.

export interface About {
  version: string;
  /** `null` before the watcher has created the data folder. */
  data_bytes: number | null;
}

export function about(): Promise<About> {
  return invoke("about");
}

export function openDataFolder(): Promise<void> {
  return invoke("open_data_folder");
}

/** Saves one change on top of the current settings and applies its appearance at once. */
export async function saveSettings(change: Partial<Settings>): Promise<void> {
  const current = app.settings?.settings;
  if (!current) return;
  await app.saveSettings({ ...current, ...change });
}

export type RetentionField = keyof Retention;

/** What the watcher accepts for each kind of history, as the core checks it. */
export const LIMITS: Record<RetentionField, readonly [number, number]> = {
  samples_hours: [24, 168],
  minutes_days: [30, 365],
  records_days: [90, 730],
};

/** The choices each select offers; a stored value outside them is added. */
export const CHOICES: Record<RetentionField, readonly number[]> = {
  samples_hours: [24, 48, 72, 168],
  minutes_days: [30, 90, 180, 365],
  records_days: [90, 180, 365, 730],
};

const MB = 1024 * 1024;

/** Predicted bytes per kind: 35 MB a day of seconds, 0.3 MB a day of minutes, 20 MB a year of records. */
export function predictedBytes(retention: Retention): Record<RetentionField, number> {
  return {
    samples_hours: (retention.samples_hours / 24) * 35 * MB,
    minutes_days: retention.minutes_days * 0.3 * MB,
    records_days: (retention.records_days / 365) * 20 * MB,
  };
}

const UNITS = ["kilobyte", "megabyte", "gigabyte"] as const;

/** "96 MB", "1,2 GB": binary multiples, as File Explorer counts them. */
export function formatSize(bytes: number): string {
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return new Intl.NumberFormat(lang(), {
    style: "unit",
    unit: UNITS[unit],
    maximumFractionDigits: value < 10 ? 1 : 0,
  }).format(value);
}
