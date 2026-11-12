// The About dialog in the browser. `?about=stale` has a monitor of an older protocol running,
// `?about=stopped` a stopped one, `?about=bare` no helper, Windows or WebView2 read.
import type { AboutVersions } from "../../lib/about";
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);
const mode = query.get("about") ?? "ok";

function versions(): AboutVersions {
  const facts: AboutVersions = {
    version: "0.1.1",
    released: "2026-11-11",
    monitor: { run: "running", version: "0.1.1", protocol: null },
    helper: "0.1.1",
    schema: 13,
    protocol: 22,
    windows: { edition: "Windows 11 Pro", family: "Windows 11", release: "24H2", build: "26100.2605", version: "10.0.26100.2605" },
    webview: "131.0.2903.112",
  };
  if (mode === "stale") return { ...facts, monitor: { run: "other", version: null, protocol: 13 } };
  if (mode === "stopped") return { ...facts, monitor: { run: "stopped", version: "0.1.1", protocol: null } };
  if (mode === "bare") return { ...facts, released: null, helper: null, windows: null, webview: null };
  return facts;
}

export const handlers: Record<string, Handler> = {
  about_versions: () => versions(),
  open_page: () => null,
};
