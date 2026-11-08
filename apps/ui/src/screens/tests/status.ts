import type { MarkKind } from "../../components/ConcernMark.svelte";
import type { StatusKind } from "../../components/StatusMark.svelte";
import { t } from "../../lib/i18n.svelte";

/** The status symbol and word for how a device or a test stands. */
export function statusOf(mark: MarkKind): { kind: StatusKind; word: string } {
  const kind: StatusKind =
    mark === "Critical" ? "critical" : mark === "Attention" || mark === "Partial" ? "attention" : mark === "Ok" ? "ok" : "info";
  const word = mark === "Critical" || mark === "Ok" || mark === "Partial" ? t(`ui.tests.mark.${mark}`) : t(`ui.health.mark.${mark}`);
  return { kind, word };
}
