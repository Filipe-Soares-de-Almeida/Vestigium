import type { AgentStatus, FindingKind, MonitorSource, SourceFailure } from "./types";

/** A health check whose last success is this old, with a later attempt failed, counts as not verified. */
export const HEALTH_STALE_MS = 3 * 60 * 60_000;

/**
 * When the health check last finished, if it has failed since and that was long enough ago to
 * matter; a check that never finished counts from the start of the watching. `null` otherwise.
 */
export function unverifiedSince(status: AgentStatus, now: number): number | null {
  const { health_attempt_ms: attempt, health_ok_ms: ok } = status.monitor;
  if (attempt === null) return null;
  if (ok !== null && ok >= attempt) return null;
  const since = ok ?? status.started_at_ms;
  return now - since >= HEALTH_STALE_MS ? since : null;
}

/** The catalog key of the name of a source. */
export function sourceKey(source: MonitorSource): string {
  return `ui.monitor.source.${source}`;
}

/** The earliest moment a source of the list began to fail; `null` for an empty list. */
export function failingSince(failing: SourceFailure[]): number | null {
  return failing.length ? Math.min(...failing.map((f) => f.since_ms)) : null;
}

/** Whether the pass could not read this kind of finding. */
export function unreadKind(status: AgentStatus | null, kind: FindingKind): boolean {
  return status?.monitor.health_unread.includes(kind) ?? false;
}
