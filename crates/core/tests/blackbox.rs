mod common;

use std::collections::HashSet;

use common::{GIB, T, calm, event, machine};
use vestigium_core::{
    Blackbox, Cause, EpisodeKind, EventMeaning, Gap, GapReason, HungWindow, Occurrence,
    OccurrenceKind, Output, ProbeTick, Severity, Timing, Trigger,
};

fn blackbox() -> Blackbox {
    Blackbox::new(machine(), Timing::default())
}

/// Feeds one calm sample per second from `from_s` to `to_s` relative to `T`, inclusive.
fn feed(bb: &mut Blackbox, from_s: i64, to_s: i64) {
    for s in from_s..=to_s {
        bb.push_sample(calm(T + s * 1_000));
    }
}

fn late(bb: &mut Blackbox, wall_ms: i64, lag_ms: f32) {
    bb.push_probe(ProbeTick { wall_ms, lag_ms });
}

fn occurrences(outputs: Vec<Output>) -> Vec<Occurrence> {
    outputs
        .into_iter()
        .filter_map(|o| match o {
            Output::Occurrence(occurrence) => Some(*occurrence),
            _ => None,
        })
        .collect()
}

fn hung(pid: u32, name: &str) -> HungWindow {
    HungWindow {
        pid,
        name: name.to_string(),
    }
}

#[test]
fn freeze_becomes_an_occurrence_after_the_post_window() {
    let mut bb = blackbox();
    feed(&mut bb, -70, 2);
    late(&mut bb, T + 2_000, 2_000.0);
    feed(&mut bb, 3, 31);

    assert!(occurrences(bb.poll(T + 31_999)).is_empty());
    let found = occurrences(bb.poll(T + 32_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, OccurrenceKind::Freeze);
    assert_eq!((found[0].start_ms, found[0].end_ms), (T, T + 2_000));
    assert_eq!(found[0].severity, Severity::Medium);
    assert!(!found[0].truncated);
}

#[test]
fn window_spans_sixty_seconds_before_and_thirty_after() {
    let mut bb = blackbox();
    feed(&mut bb, -90, 40);
    late(&mut bb, T + 2_000, 2_000.0);

    let found = occurrences(bb.poll(T + 40_000));
    let window = &found[0].window;
    assert_eq!((window.from_ms, window.to_ms), (T - 60_000, T + 32_000));
    assert_eq!(window.samples.first().map(|s| s.ts_ms), Some(T - 60_000));
    assert_eq!(window.samples.last().map(|s| s.ts_ms), Some(T + 32_000));
    assert_eq!(window.samples.len(), 93);
}

#[test]
fn close_triggers_merge_into_one_occurrence() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 1_000, 1_000.0);
    late(&mut bb, T + 21_000, 1_000.0);
    late(&mut bb, T + 60_000, 500.0);

    let found = occurrences(bb.poll(T + 200_000));
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].triggers.len(), 2);
    assert_eq!((found[0].start_ms, found[0].end_ms), (T, T + 21_000));
    assert_eq!(found[1].start_ms, T + 59_500);
}

#[test]
fn merging_stops_at_five_minutes() {
    let mut bb = blackbox();
    for k in 0..21 {
        late(&mut bb, T + k * 20_000 + 500, 500.0);
    }

    let found = occurrences(bb.poll(T + 1_000_000));
    assert!(found.len() >= 2);
    for occurrence in &found {
        assert!(occurrence.end_ms - occurrence.start_ms <= 300_000);
    }
    let triggers: usize = found.iter().map(|o| o.triggers.len()).sum();
    assert_eq!(triggers, 21);
}

#[test]
fn sleep_gap_is_not_a_freeze() {
    let gap = Gap {
        start_ms: T,
        end_ms: T + 40_000,
        reason: GapReason::Suspended,
    };
    let mut gap_first = blackbox();
    feed(&mut gap_first, -60, 0);
    gap_first.push_gap(gap);
    late(&mut gap_first, T + 40_500, 40_000.0);
    assert!(occurrences(gap_first.poll(T + 200_000)).is_empty());

    let mut tick_first = blackbox();
    feed(&mut tick_first, -60, 0);
    late(&mut tick_first, T + 40_500, 40_000.0);
    tick_first.push_gap(gap);
    let outputs = tick_first.poll(T + 200_000);
    assert!(outputs.contains(&Output::Gap(gap)));
    assert!(occurrences(outputs).is_empty());
}

#[test]
fn hang_right_after_a_freeze_is_dropped() {
    let mut bb = blackbox();
    let running: HashSet<u32> = [5, 6].into();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 6_000, 6_000.0);
    bb.push_hangs(T + 7_000, &[hung(5, "editor.exe")], &running);
    bb.push_hangs(T + 9_000, &[], &running);

    let found = occurrences(bb.poll(T + 100_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, OccurrenceKind::Freeze);
    assert!(
        found[0]
            .triggers
            .iter()
            .all(|t| matches!(t, Trigger::Freeze { .. }))
    );
    assert!(
        found[0]
            .suspects
            .iter()
            .all(|s| !matches!(s.cause, Cause::App { .. }))
    );
    assert_eq!(
        found[0].window.hangs.len(),
        1,
        "the hang stays in the story"
    );
}

#[test]
fn app_hang_alone_names_the_app() {
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    feed(&mut bb, -60, 0);
    for s in (0..=14).step_by(2) {
        bb.push_hangs(T + s * 1_000, &[hung(11, "player.exe")], &running);
    }
    bb.push_hangs(T + 16_000, &[], &running);

    let found = occurrences(bb.poll(T + 100_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, OccurrenceKind::AppHang);
    assert_eq!(found[0].start_ms, T - 5_000);
    assert_eq!(
        found[0].suspects[0].cause,
        Cause::App {
            name: "player.exe".to_string()
        }
    );
}

#[test]
fn storage_reset_event_is_an_occurrence() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    bb.push_event(event(T, "stornvme", 129, EventMeaning::StorageReset));
    bb.push_event(event(
        T - 3_600_000,
        "storahci",
        129,
        EventMeaning::StorageReset,
    ));

    let found = occurrences(bb.poll(T + 100_000));
    assert_eq!(
        found.len(),
        1,
        "events from before watching started stay marks"
    );
    assert_eq!(found[0].kind, OccurrenceKind::Reported);
}

#[test]
fn drain_closes_pending_and_marks_truncated() {
    let mut bb = blackbox();
    for s in -130..=0 {
        let mut sample = calm(T + s * 1_000);
        sample.mem_avail_bytes = GIB / 2;
        bb.push_sample(sample);
    }
    late(&mut bb, T + 2_000, 2_000.0);

    let outputs = bb.drain(T + 5_000);
    assert!(
        outputs
            .iter()
            .any(|o| matches!(o, Output::Episode(e) if e.truncated))
    );
    let found = occurrences(outputs);
    assert_eq!(found.len(), 1);
    assert!(found[0].truncated);
    assert_eq!(found[0].window.to_ms, T + 5_000);
    assert!(occurrences(bb.poll(T + 100_000)).is_empty());
}

#[test]
fn rings_keep_ten_minutes() {
    let mut bb = blackbox();
    feed(&mut bb, 0, 900);
    bb.poll(T + 900_000);

    let oldest = bb.samples().next().map(|s| s.ts_ms);
    assert_eq!(oldest, Some(T + 300_000));
    assert_eq!(bb.samples().count(), 601);
}

#[test]
fn recent_state_counts_the_last_day() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 6_000, 6_000.0);
    late(&mut bb, T + 200_000, 400.0);
    bb.poll(T + 300_000);

    let state = bb.recent(T + 3_600_000);
    assert_eq!((state.occurrences, state.severe), (2, 1));
    assert_eq!(bb.recent(T + 200_000 + 86_400_000).occurrences, 0);
}

#[test]
fn pressure_lasts_until_drained() {
    let mut bb = blackbox();
    for s in 0..=130 {
        let mut sample = calm(T + s * 1_000);
        sample.mem_avail_bytes = GIB / 2;
        bb.push_sample(sample);
    }
    assert_eq!(bb.open_pressure(), [EpisodeKind::MemoryHigh]);

    bb.drain(T + 131_000);
    assert!(bb.open_pressure().is_empty());
}
