mod common;

use common::{GIB, T, event, quiet_window, tweak};
use vestigium_core::{
    Cause, Confidence, CrashKind, EventMeaning, FactKind, OccurrenceKind, POSTMORTEM_BEFORE_MS,
    POSTMORTEM_REACH_MS, PowerLossDetail, Severity, WinEvent, Window, postmortem,
};

/// What was stored up to the last second before the fall at T.
fn before_fall() -> Window {
    let mut window = quiet_window();
    window.samples.retain(|s| s.ts_ms <= T);
    window.tops.retain(|t| t.ts_ms <= T);
    window.to_ms = T;
    window
}

/// The Kernel-Power 41 and the 6008 of the next boot, ten minutes later.
fn next_boot() -> Vec<WinEvent> {
    vec![
        event(
            T + 600_000,
            "Microsoft-Windows-Kernel-Power",
            41,
            EventMeaning::PowerLoss,
        ),
        event(
            T + 610_000,
            "EventLog",
            6008,
            EventMeaning::UnexpectedShutdown,
        ),
    ]
}

fn detail(bugcheck_code: u32, power_state: u8, power_button: bool) -> PowerLossDetail {
    PowerLossDetail {
        bugcheck_code,
        bugcheck_params: [1, 2, 3, 4],
        power_state,
        power_button,
    }
}

#[test]
fn crash_kind_follows_the_detail() {
    assert_eq!(
        detail(0x9f, 6, true).crash_kind(),
        CrashKind::BlueScreen {
            code: 0x9f,
            params: [1, 2, 3, 4]
        }
    );
    assert_eq!(detail(0, 6, true).crash_kind(), CrashKind::PowerButton);
    assert_eq!(
        detail(0, 6, false).crash_kind(),
        CrashKind::WhileShuttingDown
    );
    assert_eq!(detail(0, 4, false).crash_kind(), CrashKind::WhileSleeping);
    assert_eq!(detail(0, 5, false).crash_kind(), CrashKind::WhileSleeping);
    assert_eq!(detail(0, 0, false).crash_kind(), CrashKind::NoTrace);
    assert_eq!(detail(0, 1, false).crash_kind(), CrashKind::NoTrace);
}

#[test]
fn blue_screen_is_a_severe_post_mortem() {
    let kind = CrashKind::BlueScreen {
        code: 0x50,
        params: [0; 4],
    };

    let found = postmortem(before_fall(), T, T + 20_000, kind, next_boot()).unwrap();

    assert_eq!(found.kind, OccurrenceKind::PostMortem);
    assert_eq!(found.severity, Severity::Severe);
    assert_eq!((found.start_ms, found.end_ms), (T, T + 20_000));
    assert!(found.truncated);
    let top = &found.suspects[0];
    assert_eq!(top.cause, Cause::BlueScreen { code: 0x50 });
    assert_eq!(top.confidence, Confidence::High);
    assert_eq!(top.evidence.len(), 2, "the 41 and the 6008");
    for suspect in &found.suspects {
        assert!(!suspect.evidence.is_empty(), "{:?}", suspect.cause);
    }
}

#[test]
fn a_shutdown_that_did_not_finish_is_no_occurrence() {
    assert_eq!(
        postmortem(
            before_fall(),
            T,
            T + 5_000,
            CrashKind::WhileShuttingDown,
            next_boot()
        ),
        None
    );
}

#[test]
fn a_fall_far_from_the_last_sample_is_no_occurrence() {
    let far = T + POSTMORTEM_REACH_MS + 1;
    assert_eq!(
        postmortem(before_fall(), T, far, CrashKind::NoTrace, next_boot()),
        None
    );
    let stamped_earlier = T - 50_000;
    assert!(
        postmortem(
            before_fall(),
            T,
            stamped_earlier,
            CrashKind::NoTrace,
            next_boot()
        )
        .is_some(),
        "the 6008 time can come a little before the last sample"
    );
}

#[test]
fn a_fall_with_no_trace_names_the_missing_dump() {
    let mut events = next_boot();
    events.push(event(T + 605_000, "volmgr", 46, EventMeaning::NoCrashDump));

    let found = postmortem(before_fall(), T, T + 1_000, CrashKind::NoTrace, events).unwrap();

    let cut = found
        .suspects
        .iter()
        .find(|s| matches!(s.cause, Cause::PowerCut { .. }))
        .unwrap();
    assert_eq!(cut.cause, Cause::PowerCut { no_dump: true });
    assert_eq!(cut.confidence, Confidence::Low);
    assert_eq!(cut.evidence.len(), 3);
}

#[test]
fn a_fatal_hardware_error_before_the_fall_comes_first() {
    let mut window = before_fall();
    window.events.push(event(
        T - 3_000,
        "Microsoft-Windows-WHEA-Logger",
        18,
        EventMeaning::HardwareError,
    ));

    let found = postmortem(window, T, T + 1_000, CrashKind::NoTrace, next_boot()).unwrap();

    assert_eq!(found.suspects[0].cause, Cause::HardwareFault);
    assert_eq!(found.suspects[0].confidence, Confidence::High);
}

#[test]
fn disk_and_display_events_minutes_before_the_fall_name_their_suspects() {
    let mut window = before_fall();
    window
        .events
        .push(event(T - 90_000, "disk", 153, EventMeaning::DiskRetry));
    window.events.push(event(
        T - 60_000,
        "Display",
        4101,
        EventMeaning::DisplayReset,
    ));

    let found = postmortem(window, T, T + 1_000, CrashKind::NoTrace, next_boot()).unwrap();

    let causes: Vec<&Cause> = found.suspects.iter().map(|s| &s.cause).collect();
    assert!(causes.iter().any(|c| matches!(c, Cause::Storage { .. })));
    assert!(causes.contains(&&Cause::GpuDriver));
}

#[test]
fn events_older_than_the_stored_window_name_no_suspect() {
    let mut window = before_fall();
    window.events.push(event(
        T - POSTMORTEM_BEFORE_MS - 1_000,
        "disk",
        153,
        EventMeaning::DiskRetry,
    ));

    let found = postmortem(window, T, T + 1_000, CrashKind::NoTrace, next_boot()).unwrap();

    assert!(
        found
            .suspects
            .iter()
            .all(|s| !matches!(s.cause, Cause::Storage { .. }))
    );
}

#[test]
fn power_button_and_memory_running_out() {
    let mut window = before_fall();
    tweak(&mut window, -20, 0, |s| s.mem_avail_bytes = GIB / 2);

    let found = postmortem(window, T, T + 30_000, CrashKind::PowerButton, next_boot()).unwrap();

    assert!(
        found
            .suspects
            .iter()
            .any(|s| s.cause == Cause::Unresponsive)
    );
    assert!(
        found
            .suspects
            .iter()
            .any(|s| matches!(s.cause, Cause::Memory { .. })),
        "the ordinary rules still read the seconds before the fall"
    );
}

#[test]
fn facts_mark_the_last_second_and_the_fall() {
    let found = postmortem(
        before_fall(),
        T,
        T + 9_000,
        CrashKind::WhileSleeping,
        next_boot(),
    )
    .unwrap();

    let kinds: Vec<_> = found
        .facts
        .iter()
        .filter(|f| matches!(f.kind, FactKind::LastWatched | FactKind::WentDown { .. }))
        .map(|f| (f.at_ms, f.kind.clone()))
        .collect();
    assert_eq!(
        kinds,
        [
            (T, FactKind::LastWatched),
            (
                T + 9_000,
                FactKind::WentDown {
                    kind: CrashKind::WhileSleeping
                }
            ),
        ]
    );
}
