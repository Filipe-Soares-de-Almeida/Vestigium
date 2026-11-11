import { formatClock } from "../../lib/format";
import { t } from "../../lib/i18n.svelte";

/** The live part a notice stands over. */
export type SilentPart = "chart" | "top" | "hung";

/** Says why a live part shows old values, or none, while the monitor does not answer. */
export function silentNotice(part: SilentPart, hasData: boolean, sinceMs: number): string {
  if (part === "hung") return t("ui.now.silent.hung");
  const time = formatClock(sinceMs);
  if (!hasData) return t("ui.now.silent.nothing", { time });
  return t(part === "chart" ? "ui.now.silent.chart" : "ui.now.silent.top", { time });
}
