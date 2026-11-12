import type { AgentLink, AgentStatus, FindingKind, MonitorSource, SourceFailure } from "./types";

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

/** How the checks of the hourly pass stand, as the Health screen needs them. */
export interface PassState {
  /** No pass has finished lately, or the monitor does not answer: nothing it says is current. */
  stale: boolean;
  /** When the last pass finished, if that is known; `null` when the monitor is silent. */
  since: number | null;
  /** The monitor does not answer, so the list is only the last one saved. */
  silent: boolean;
  /** Kinds the last finished pass could not read. */
  unread: FindingKind[];
}

/**
 * The state of the hourly pass from what the watcher said last; `null` when it says nothing about
 * it (stopped, not the same version, or not heard yet), so the screen shows nothing new.
 */
export function passState(link: AgentLink | null, now: number): PassState | null {
  if (link === null) return null;
  if (link.link === "silent") return { stale: true, since: null, silent: true, unread: [] };
  if (link.link !== "running") return null;
  const since = unverifiedSince(link, now);
  return { stale: since !== null, since, silent: false, unread: link.monitor.health_unread };
}
