use crate::{
    CrashKind, Occurrence, OccurrenceKind, Severity, Trigger, WinEvent, Window, facts, suspects,
};

/// Farthest the fall may be from the last stored sample for the watcher to have seen it coming.
pub const POSTMORTEM_REACH_MS: i64 = 3 * 60_000;
/// How much of what was stored before the fall a post-mortem keeps.
pub const POSTMORTEM_BEFORE_MS: i64 = 120_000;

/// The occurrence of a PC that went down while watched: `window` holds what was stored up to
/// `last_ms`, `down_ms` is when Windows says it fell, and `events` are those its next boot logged.
/// `None` for a shutdown that did not finish, or a fall too far from the last sample.
pub fn postmortem(
    mut window: Window,
    last_ms: i64,
    down_ms: i64,
    kind: CrashKind,
    events: Vec<WinEvent>,
) -> Option<Occurrence> {
    if kind == CrashKind::WhileShuttingDown || (down_ms - last_ms).abs() > POSTMORTEM_REACH_MS {
        return None;
    }
    let end_ms = down_ms.max(last_ms);
    window.events.extend(events);
    window.to_ms = window.to_ms.max(end_ms);
    let triggers = vec![Trigger::Crash {
        last_ms,
        down_ms,
        kind,
    }];
    Some(Occurrence {
        kind: OccurrenceKind::PostMortem,
        start_ms: last_ms,
        end_ms,
        severity: Severity::Severe,
        suspects: suspects(&triggers, &window),
        facts: facts(&triggers, &window),
        triggers,
        window,
        sensitive: false,
        truncated: true,
    })
}
