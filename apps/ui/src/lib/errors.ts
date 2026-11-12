import { t } from "./i18n.svelte";
import type { UiErrorKind } from "./types";

const KINDS: readonly UiErrorKind[] = [
  "agent_stopped",
  "mismatch",
  "refused",
  "start_timeout",
  "store",
  "ipc",
  "failed",
  "declined",
  "changed_since",
  "helper_timed_out",
  "cancelled",
  "capture_running",
];

/** The kind of a rejected command; anything that is not a `UiError` counts as `failed`. */
export function errorKind(error: unknown): UiErrorKind {
  const kind = (error as { kind?: unknown } | null)?.kind;
  return KINDS.includes(kind as UiErrorKind) ? (kind as UiErrorKind) : "failed";
}

/** A sentence for a rejected command, in the window's language. */
export function errorText(error: unknown): string {
  return t(`ui.common.error.${errorKind(error)}`);
}
