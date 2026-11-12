// What the About dialog reads from the window's Rust side, and the lists it builds from it.
import { invoke } from "@tauri-apps/api/core";

import shipped from "../../src-tauri/third-party.json";
import { lang, t } from "./i18n.svelte";

/** Whether the monitor runs: `other` speaks another protocol, or the pipe has another owner. */
export type MonitorRun = "running" | "stopped" | "other";

export interface MonitorVersion {
  run: MonitorRun;
  /** From the handshake while it runs, from `Vestigium.exe` while it is stopped. */
  version: string | null;
  /** The protocol it speaks, when it refused this window's. */
  protocol: number | null;
}

export interface WindowsVersion {
  /** "Windows 11 Pro". */
  edition: string;
  /** "Windows 11", or the whole edition for a product that is neither 10 nor 11. */
  family: string;
  /** "24H2"; `null` before Windows 10 20H2. */
  release: string | null;
  /** "26100.2605". */
  build: string;
  /** "10.0.26100.2605". */
  version: string;
}

export interface AboutVersions {
  version: string;
  /** "2026-11-11", from the bundled changelog. */
  released: string | null;
  monitor: MonitorVersion;
  /** The file version of the helper next to the window; `null` when it is missing. */
  helper: string | null;
  schema: number;
  protocol: number;
  windows: WindowsVersion | null;
  webview: string | null;
}

/** The pages the dialog opens in the browser; the window's Rust side holds their addresses. */
export type Page = "repository" | "wiki" | "privacy" | "license" | "author";

export function aboutVersions(): Promise<AboutVersions> {
  return invoke("about_versions");
}

export function openPage(page: Page): Promise<void> {
  return invoke("open_page", { page });
}

/** The author's name as the credits show it. */
export const AUTHOR = "Filipe Soares de Almeida";

export const FILES = { window: "Vestigium.Window.exe", monitor: "Vestigium.exe", helper: "Vestigium.Helper.exe" };

/** A version that differs from the window's, or `null` when it is the same or unknown. */
export function otherVersion(version: string | null, app: string): string | null {
  return version !== null && version !== app ? version : null;
}

/** Whether `a` is an older "major.minor.patch" than `b`. */
function older(a: string, b: string): boolean {
  const pa = a.split(/[.+-]/).map(Number);
  const pb = b.split(/[.+-]/).map(Number);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) < (pb[i] ?? 0);
  }
  return false;
}

/** The sentence under the list when the running monitor is not this window's version, or `null`. */
export function monitorHint(facts: AboutVersions): string | null {
  const { monitor } = facts;
  if (monitor.run === "other") {
    const earlier = monitor.protocol === null || monitor.protocol < facts.protocol;
    return t(earlier ? "ui.about.monitor.earlier" : "ui.about.monitor.another");
  }
  const other = monitor.run === "running" ? otherVersion(monitor.version, facts.version) : null;
  if (other === null) return null;
  return t(older(other, facts.version) ? "ui.about.monitor.earlier" : "ui.about.monitor.another");
}

/** A license expression as declared, with its operators in words: "Apache-2.0 or MIT". */
export function licenseText(expression: string): string {
  if (expression.trim().toLowerCase() === "blessing") return t("ui.about.license.public_domain");
  return expression
    .replace(/\s*\/\s*/g, " OR ")
    .replace(/ OR /g, ` ${t("ui.about.license.or")} `)
    .replace(/ AND /g, ` ${t("ui.about.license.and")} `);
}

/** One shipped component, as `third-party.json` lists it. */
export interface Component {
  name: string;
  version: string;
  license: string;
  programs: string[];
  main: boolean;
}

/** One row of the components table. */
export interface ComponentRow {
  key: string;
  name: string;
  caption: string | null;
  version: string;
  license: string;
}

export const COMPONENTS: readonly Component[] = shipped;

/** The entry for Rust's own library, which is not a package and so has a name to translate. */
const STD = "Rust standard library";

/** The main components in the order the credits list them; one missing here sorts after them. */
export const MAIN_ORDER = [
  "tauri",
  "wry",
  "tao",
  "webview2-com",
  "tauri-plugin-updater",
  "tauri-plugin-single-instance",
  "@tauri-apps/api",
  "svelte",
  "uplot",
  "windows",
  "rusqlite",
  "SQLite",
  "tray-icon",
  "serde",
  "serde_json",
  "postcard",
  "miniz_oxide",
  "thiserror",
  "PresentMon",
  STD,
];

/** The catalog key of a main component's name; its caption adds `.caption`. */
export function componentKey(name: string): string {
  return `ui.about.component.${name.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "")}`;
}

/** The catalog's text for `key`, or `null` when the catalog has none. */
function optional(key: string): string | null {
  const text = t(key);
  return text === key ? null : text;
}

function row(component: Component, main: boolean): ComponentRow {
  const key = componentKey(component.name);
  const named = main || component.name === STD;
  return {
    key: `${component.name}@${component.version}`,
    name: named ? (optional(key) ?? component.name) : component.name,
    caption: main ? optional(`${key}.caption`) : null,
    version: component.version,
    license: licenseText(component.license),
  };
}

/** The components Vestigium uses directly, with their display names and captions. */
export function mainRows(list: readonly Component[] = COMPONENTS): ComponentRow[] {
  const rank = (name: string) => {
    const at = MAIN_ORDER.indexOf(name);
    return at < 0 ? MAIN_ORDER.length : at;
  };
  return list
    .filter((c) => c.main)
    .sort((a, b) => rank(a.name) - rank(b.name) || a.name.localeCompare(b.name))
    .map((c) => row(c, true));
}

/** Every shipped component under its declared name, in alphabetical order. */
export function allRows(list: readonly Component[] = COMPONENTS): ComponentRow[] {
  const bare = (name: string) => name.replace(/^@/, "");
  return list
    .map((c) => row(c, false))
    .sort((a, b) => bare(a.name).localeCompare(bare(b.name), lang(), { sensitivity: "base" }));
}

/** The rows whose name holds `query`, ignoring case. */
export function filterRows(rows: ComponentRow[], query: string): ComponentRow[] {
  const q = query.trim().toLocaleLowerCase(lang());
  return q ? rows.filter((r) => r.name.toLocaleLowerCase(lang()).includes(q)) : rows;
}
