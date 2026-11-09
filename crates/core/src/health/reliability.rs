use crate::{EventMeaning, WinEvent};

/// Meanings that count as a failure, as the Reliability Monitor counts them; the 6008 that comes
/// with every Kernel-Power 41 is left out so one fall counts once.
const FAILURES: [EventMeaning; 7] = [
    EventMeaning::PowerLoss,
    EventMeaning::BugCheck,
    EventMeaning::AppCrash,
    EventMeaning::AppHang,
    EventMeaning::ServiceCrash,
    EventMeaning::HardwareError,
    EventMeaning::DisplayReset,
];

pub fn is_failure(meaning: EventMeaning) -> bool {
    FAILURES.contains(&meaning)
}

/// Failures logged in `from_ms..to_ms`.
pub fn failures_between(events: &[WinEvent], from_ms: i64, to_ms: i64) -> u32 {
    events
        .iter()
        .filter(|e| (from_ms..to_ms).contains(&e.ts_ms) && is_failure(e.meaning))
        .count() as u32
}
