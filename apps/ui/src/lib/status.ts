import { app } from "./app.svelte";
import { captures } from "./capture.svelte";

export type ShellTone = "ok" | "paused" | "off";

/** How the watcher stands, for the sidebar and the home screen; read it inside `$derived`. */
export function shellStatus(): { tone: ShellTone; key: string } {
  if (app.link === null) return { tone: "off", key: "ui.shell.status.connecting" };
  if (app.paused) return { tone: "paused", key: "ui.shell.status.paused" };
  if (captures.run !== null) return { tone: "ok", key: "ui.shell.status.capturing" };
  if (app.running && app.link.link === "running" && app.link.sensitive_until_ms !== null) {
    return { tone: "ok", key: "ui.shell.status.sensitive" };
  }
  if (app.running) return { tone: "ok", key: "ui.shell.status.watching" };
  if (app.stopped) return { tone: "off", key: "ui.shell.status.stopped" };
  return { tone: "off", key: "ui.shell.status.unreachable" };
}
