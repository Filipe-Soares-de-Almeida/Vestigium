// What the About dialog reads from the window's Rust side.

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
