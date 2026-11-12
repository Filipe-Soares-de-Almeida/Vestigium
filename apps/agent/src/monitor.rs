//! How the agent tells whether it is reading: the state the tray and the window show.

use vestigium_ipc::{AgentStatus, MonitorState, MonitorStatus};

/// Without a sample for this long, and nothing paused, the agent says it has no reading.
pub const NO_READING_AFTER_MS: i64 = 5_000;
/// A health check whose last success is this old, with a later attempt failed, is not verified.
pub const HEALTH_STALE_MS: i64 = 3 * 60 * 60_000;

/// When the health check last finished, if it has failed since and that was long enough ago to
/// matter; a check that never finished counts from the start of the watching.
pub fn unverified_since(status: &AgentStatus, now_ms: i64) -> Option<i64> {
    let monitor = &status.monitor;
    let attempt = monitor.health_attempt_ms?;
    if monitor.health_ok_ms.is_some_and(|ok| ok >= attempt) {
        return None;
    }
    let since = monitor.health_ok_ms.unwrap_or(status.started_at_ms);
    (now_ms.saturating_sub(since) >= HEALTH_STALE_MS).then_some(since)
}

/// The status as the agent answers it at `now_ms`: the stored fields and the state worked out from
/// them, so a sampler that stopped writing still shows.
pub fn view(status: &AgentStatus, now_ms: i64) -> MonitorStatus {
    let mut monitor = status.monitor.clone();
    monitor.no_reading_since_ms = None;
    if status.paused || status.suspended {
        monitor.state = MonitorState::Watching;
        return monitor;
    }
    let last = [
        Some(status.started_at_ms),
        status.last_sample_ms,
        monitor.resumed_ms,
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(status.started_at_ms);
    monitor.state = if now_ms.saturating_sub(last) > NO_READING_AFTER_MS {
        monitor.no_reading_since_ms = Some(last);
        MonitorState::NoReading
    } else if monitor.failing.is_empty() {
        MonitorState::Watching
    } else {
        MonitorState::Degraded
    };
    monitor
}

#[cfg(test)]
mod tests {
    use vestigium_ipc::{MonitorSource, SourceFailure};

    use super::*;

    const NOW: i64 = 1_000_000;

    fn status() -> AgentStatus {
        AgentStatus {
            started_at_ms: NOW - 600_000,
            last_sample_ms: Some(NOW - 1_000),
            ..AgentStatus::default()
        }
    }

    fn failing() -> SourceFailure {
        SourceFailure {
            source: MonitorSource::Counters,
            since_ms: NOW - 60_000,
            error: "counters closed".to_string(),
        }
    }

    #[test]
    fn a_fresh_sample_is_watching() {
        let monitor = view(&status(), NOW);

        assert_eq!(monitor.state, MonitorState::Watching);
        assert_eq!(monitor.no_reading_since_ms, None);
    }

    #[test]
    fn an_old_sample_is_no_reading_since_that_sample() {
        let mut stale = status();
        stale.last_sample_ms = Some(NOW - 6_000);
        stale.monitor.failing.push(failing());

        let monitor = view(&stale, NOW);

        assert_eq!(monitor.state, MonitorState::NoReading);
        assert_eq!(monitor.no_reading_since_ms, Some(NOW - 6_000));
    }

    #[test]
    fn a_start_with_no_sample_counts_from_the_start() {
        let mut fresh = status();
        fresh.last_sample_ms = None;
        fresh.started_at_ms = NOW - 2_000;
        assert_eq!(view(&fresh, NOW).state, MonitorState::Watching);

        fresh.started_at_ms = NOW - 30_000;
        let monitor = view(&fresh, NOW);

        assert_eq!(monitor.state, MonitorState::NoReading);
        assert_eq!(monitor.no_reading_since_ms, Some(NOW - 30_000));
    }

    #[test]
    fn a_resume_gives_the_sampler_a_few_seconds() {
        let mut resumed = status();
        resumed.last_sample_ms = Some(NOW - 3_600_000);
        resumed.monitor.resumed_ms = Some(NOW - 1_000);

        assert_eq!(view(&resumed, NOW).state, MonitorState::Watching);
    }

    #[test]
    fn paused_and_suspended_are_never_no_reading() {
        let mut old = status();
        old.last_sample_ms = Some(NOW - 600_000);
        old.monitor.failing.push(failing());

        let mut paused = old.clone();
        paused.paused = true;
        let mut suspended = old;
        suspended.suspended = true;

        assert_eq!(view(&paused, NOW).state, MonitorState::Watching);
        assert_eq!(view(&suspended, NOW).state, MonitorState::Watching);
    }

    #[test]
    fn a_health_check_is_unverified_after_three_hours_without_a_success() {
        let mut checked = status();
        checked.monitor.health_ok_ms = Some(NOW - 4 * 60 * 60_000);
        assert_eq!(unverified_since(&checked, NOW), None, "no attempt since");

        checked.monitor.health_attempt_ms = Some(NOW - 60_000);
        assert_eq!(
            unverified_since(&checked, NOW),
            checked.monitor.health_ok_ms
        );

        checked.monitor.health_ok_ms = Some(NOW - 2 * 60 * 60_000);
        assert_eq!(unverified_since(&checked, NOW), None, "still fresh enough");

        checked.monitor.health_ok_ms = Some(NOW - 30_000);
        assert_eq!(
            unverified_since(&checked, NOW),
            None,
            "a success after the attempt"
        );

        checked.monitor.health_ok_ms = None;
        checked.started_at_ms = NOW - 5 * 60 * 60_000;
        assert_eq!(unverified_since(&checked, NOW), Some(NOW - 5 * 60 * 60_000));
    }

    #[test]
    fn a_failing_source_with_fresh_samples_is_degraded() {
        let mut partial = status();
        partial.monitor.failing.push(failing());

        let monitor = view(&partial, NOW);

        assert_eq!(monitor.state, MonitorState::Degraded);
        assert_eq!(monitor.failing.len(), 1);
    }
}
