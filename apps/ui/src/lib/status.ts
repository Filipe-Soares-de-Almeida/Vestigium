import { app } from "./app.svelte";
import { captures } from "./capture.svelte";
import { formatPercent, formatTime } from "./format";
import { t } from "./i18n.svelte";

export type ShellTone = "ok" | "paused" | "off" | "no_reading" | "partial" | "silent" | "unverified";

/** How the watcher stands, for the sidebar and the home screen; read it inside `$derived`. */
export function shellStatus(): { tone: ShellTone; text: string } {
  const link = app.link;
  if (link === null) return { tone: "off", text: t("ui.shell.status.connecting") };
  if (app.paused) {
    const until = app.pausedUntil;
    return {
      tone: "paused",
      text: until === null ? t("ui.shell.status.paused") : t("ui.shell.status.paused_until", { time: formatTime(until) }),
    };
  }
  if (captures.run !== null) return { tone: "ok", text: t("ui.shell.status.capturing") };
  if (app.noReading && link.link === "running") {
    const since = link.monitor.no_reading_since_ms ?? link.started_at_ms;
    return { tone: "no_reading", text: t("ui.shell.status.no_reading", { time: formatTime(since) }) };
  }
  if (app.degraded) return { tone: "partial", text: t("ui.shell.status.degraded") };
  if (app.unverified !== null) return { tone: "unverified", text: t("ui.shell.status.unverified") };
  if (app.running && link.link === "running") {
    const state = t(link.sensitive_until_ms !== null ? "ui.shell.status.sensitive" : "ui.shell.status.watching");
    const cpu = link.cpu_pct;
    return { tone: "ok", text: cpu == null ? state : t("ui.shell.status.cpu", { state, cpu: formatPercent(cpu, 1) }) };
  }
  if (app.running) return { tone: "ok", text: t("ui.shell.status.watching") };
  if (app.stopped) return { tone: "off", text: t("ui.shell.status.stopped") };
  return { tone: "off", text: t("ui.shell.status.unreachable") };
}
