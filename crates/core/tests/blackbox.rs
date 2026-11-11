mod common;

use std::collections::HashSet;

use common::{GIB, T, calm, event, machine, process};
use vestigium_core::{
    Blackbox, Cause, EpisodeKind, EventMeaning, FactKind, GAP_MARGIN_MS, Gap, GapReason,
    HungWindow, Occurrence, OccurrenceKind, Output, PressureSpan, ProbeTick, ProcessTop,
    REPORTED_EXEMPLARS, RING_MS, Severity, Timing, Trigger,
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
fn a_trigger_that_bridges_two_pending_occurrences_merges_them() {
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 500, 500.0);
    late(&mut bb, T + 50_500, 500.0);
    bb.push_hangs(T + 25_000, &[hung(11, "player.exe")], &running);
    bb.push_hangs(T + 52_000, &[], &running);

    let found = occurrences(bb.poll(T + 200_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].triggers.len(), 3);
    assert_eq!((found[0].start_ms, found[0].end_ms), (T, T + 52_000));
}

#[test]
fn an_unrelated_trigger_does_not_merge_distant_pending_occurrences() {
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 500, 500.0);
    late(&mut bb, T + 150_500, 500.0);
    bb.push_hangs(T + 25_000, &[hung(11, "player.exe")], &running);
    bb.push_hangs(T + 52_000, &[], &running);

    let found = occurrences(bb.poll(T + 300_000));
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].triggers.len(), 2);
    assert_eq!(found[1].triggers.len(), 1);
}

#[test]
fn a_bridge_does_not_merge_past_five_minutes() {
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 500, 500.0);
    late(&mut bb, T + 305_500, 500.0);
    bb.push_hangs(T + 25_000, &[hung(11, "player.exe")], &running);
    bb.push_hangs(T + 280_000, &[], &running);

    let found = occurrences(bb.poll(T + 600_000));
    assert_eq!(found.len(), 2);
    for occurrence in &found {
        assert!(occurrence.end_ms - occurrence.start_ms <= 300_000);
    }
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
fn gap_ahead_of_the_clock_stops_blocking_after_it_steps_back() {
    let gap = Gap {
        start_ms: T,
        end_ms: T + 10_800_000,
        reason: GapReason::Suspended,
    };
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    bb.push_gap(gap);
    bb.cut_gaps_to(T + 5_000);
    late(&mut bb, T + 60_000, 600.0);

    let found = occurrences(bb.poll(T + 100_000));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, OccurrenceKind::Freeze);
}

#[test]
fn gap_ahead_of_the_clock_blocks_without_the_cut() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    bb.push_gap(Gap {
        start_ms: T,
        end_ms: T + 10_800_000,
        reason: GapReason::Suspended,
    });
    late(&mut bb, T + 60_000, 600.0);

    assert!(occurrences(bb.poll(T + 100_000)).is_empty());
}

#[test]
fn cutting_gaps_leaves_a_past_gap_alone() {
    let gap = Gap {
        start_ms: T,
        end_ms: T + 40_000,
        reason: GapReason::Suspended,
    };
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    bb.push_gap(gap);
    bb.cut_gaps_to(T + 200_000);
    late(&mut bb, T + 40_500, 40_000.0);

    let outputs = bb.poll(T + 200_000);

    assert!(outputs.contains(&Output::Gap(gap)));
    assert!(occurrences(outputs).is_empty());
}

#[test]
fn cutting_gaps_removes_one_that_starts_after_now() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    bb.push_gap(Gap {
        start_ms: T + 3_600_000,
        end_ms: T + 3_700_000,
        reason: GapReason::Paused,
    });
    bb.cut_gaps_to(T + 5_000);
    late(&mut bb, T + 60_000, 600.0);

    assert_eq!(occurrences(bb.poll(T + 100_000)).len(), 1);
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
fn a_hang_still_going_after_a_resume_is_an_occurrence() {
    let gap = Gap {
        start_ms: T,
        end_ms: T + 40_000,
        reason: GapReason::Suspended,
    };
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    feed(&mut bb, -60, 0);
    bb.push_gap(gap);
    for s in (41..=75).step_by(2) {
        bb.push_hangs(T + s * 1_000, &[hung(11, "player.exe")], &running);
    }
    bb.push_hangs(T + 77_000, &[], &running);

    let found = occurrences(bb.poll(T + 200_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, OccurrenceKind::AppHang);
    assert!(found[0].start_ms > gap.end_ms + GAP_MARGIN_MS);
    assert_eq!(found[0].end_ms, T + 77_000);
}

#[test]
fn a_short_hang_right_after_a_resume_is_still_dropped() {
    let gap = Gap {
        start_ms: T,
        end_ms: T + 40_000,
        reason: GapReason::Suspended,
    };
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    feed(&mut bb, -60, 0);
    bb.push_gap(gap);
    for s in (41..=45).step_by(2) {
        bb.push_hangs(T + s * 1_000, &[hung(11, "player.exe")], &running);
    }
    bb.push_hangs(T + 47_000, &[], &running);

    assert!(occurrences(bb.poll(T + 200_000)).is_empty());
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

    let found = occurrences(bb.poll(T + 130_000));
    assert_eq!(
        found.len(),
        1,
        "events from before watching started stay marks"
    );
    assert_eq!(found[0].kind, OccurrenceKind::Reported);
}

#[test]
fn a_storm_of_one_event_keeps_few_triggers() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 330);
    for i in 0..9_000 {
        bb.push_event(event(T + i * 33, "disk", 153, EventMeaning::DiskRetry));
    }

    let found = occurrences(bb.poll(T + 600_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].triggers.len(), REPORTED_EXEMPLARS);
    assert_eq!(
        found[0].end_ms,
        T + 8_999 * 33,
        "the storm still sets the span"
    );
}

#[test]
fn a_storm_says_how_many_repeats_were_not_kept() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 330);
    for i in 0..9_000 {
        bb.push_event(event(T + i * 33, "disk", 153, EventMeaning::DiskRetry));
    }

    let found = occurrences(bb.poll(T + 600_000));
    let dropped: Vec<u32> = found[0]
        .facts
        .iter()
        .filter_map(|f| match f.kind {
            FactKind::EventsDropped { count } => Some(count),
            _ => None,
        })
        .collect();
    assert_eq!(dropped, vec![9_000 - REPORTED_EXEMPLARS as u32]);
}

#[test]
fn a_storm_does_not_push_the_start_of_the_window_out() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 120);
    bb.push_event(event(
        T - 30_000,
        "Microsoft-Windows-Kernel-Power",
        41,
        EventMeaning::PowerLoss,
    ));
    for i in 0..2_000 {
        bb.push_event(event(T + i * 30, "disk", 153, EventMeaning::DiskRetry));
    }

    let found = occurrences(bb.poll(T + 400_000));
    assert_eq!(found.len(), 1);
    let events = &found[0].window.events;
    assert!(
        events.iter().any(|e| e.id == 41),
        "the earlier event is still in the window"
    );
    let storm: Vec<i64> = events
        .iter()
        .filter(|e| e.id == 153)
        .map(|e| e.ts_ms)
        .collect();
    assert_eq!(storm.len(), REPORTED_EXEMPLARS);
    assert_eq!(storm[0], T, "the first exemplar is the first event");
}

#[test]
fn a_few_repeats_drop_nothing() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 200);
    for i in 0..REPORTED_EXEMPLARS as i64 {
        bb.push_event(event(T + i * 1_000, "disk", 153, EventMeaning::DiskRetry));
    }

    let found = occurrences(bb.poll(T + 600_000));
    assert!(
        found[0]
            .facts
            .iter()
            .all(|f| !matches!(f.kind, FactKind::EventsDropped { .. }))
    );
}

#[test]
fn a_storm_keeps_exemplars_of_each_event() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 60);
    for i in 0..200 {
        bb.push_event(event(T + i * 100, "disk", 153, EventMeaning::DiskRetry));
        bb.push_event(event(
            T + i * 100,
            "Display",
            4101,
            EventMeaning::DisplayReset,
        ));
        bb.push_event(event(
            T + i * 100,
            "stornvme",
            129,
            EventMeaning::StorageReset,
        ));
    }

    let found = occurrences(bb.poll(T + 300_000));
    assert_eq!(found.len(), 1);
    let kept = |provider: &str, id: u16| {
        found[0]
            .triggers
            .iter()
            .filter(|t| matches!(t, Trigger::Reported { event } if event.provider == provider && event.id == id))
            .count()
    };
    assert_eq!(kept("disk", 153), REPORTED_EXEMPLARS);
    assert_eq!(kept("stornvme", 129), REPORTED_EXEMPLARS);
    assert_eq!(kept("Display", 4101), REPORTED_EXEMPLARS);
}

#[test]
fn events_31_seconds_apart_do_not_make_an_occurrence_each() {
    let mut bb = blackbox();
    let mut stored = 0;
    for s in -60..3_600 {
        bb.push_sample(calm(T + s * 1_000));
        if s >= 0 && s % 31 == 0 {
            bb.push_event(event(
                T + s * 1_000,
                "stornvme",
                129,
                EventMeaning::StorageReset,
            ));
        }
        stored += occurrences(bb.poll(T + s * 1_000)).len();
    }
    stored += occurrences(bb.poll(T + 4_000_000)).len();

    assert!(
        (10..=14).contains(&stored),
        "about one occurrence per five minutes, got {stored}"
    );
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

/// Feeds a calm sample every second and a hang check every two, polling every ten seconds,
/// while an app hangs from `T` for `hang_s` seconds; returns every occurrence found.
fn long_hang(hang_s: i64) -> Vec<Occurrence> {
    let mut bb = blackbox();
    let running: HashSet<u32> = [11].into();
    let mut found = Vec::new();
    for s in -120..=hang_s + 60 {
        bb.push_sample(calm(T + s * 1_000));
        if (0..hang_s).contains(&s) && s % 2 == 0 {
            bb.push_hangs(T + s * 1_000, &[hung(11, "player.exe")], &running);
        }
        if s == hang_s {
            bb.push_hangs(T + s * 1_000, &[], &running);
        }
        if s % 10 == 0 {
            found.extend(occurrences(bb.poll(T + s * 1_000)));
        }
    }
    found
}

#[test]
fn a_window_older_than_the_ring_is_marked_truncated() {
    let found = long_hang(11 * 60);

    assert_eq!(found.len(), 1);
    assert!(found[0].window.samples.first().map(|s| s.ts_ms) > Some(found[0].window.from_ms));
    assert!(found[0].truncated);
}

#[test]
fn a_window_inside_the_ring_is_not_truncated() {
    let found = long_hang(2 * 60);

    assert_eq!(found.len(), 1);
    assert!(!found[0].truncated);
}

#[test]
fn a_window_before_the_first_sample_is_not_truncated() {
    let mut bb = blackbox();
    feed(&mut bb, 0, 40);
    late(&mut bb, T + 5_000, 2_000.0);

    let found = occurrences(bb.poll(T + 40_000));
    assert_eq!(found.len(), 1);
    assert!(!found[0].truncated);
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
fn an_output_that_was_never_stored_leaves_the_recent_count() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    late(&mut bb, T + 6_000, 6_000.0);
    late(&mut bb, T + 200_000, 400.0);
    let outputs = bb.poll(T + 300_000);
    assert_eq!(bb.recent(T + 3_600_000).occurrences, 2);

    let severe = outputs
        .iter()
        .find(|o| matches!(o, Output::Occurrence(occ) if occ.severity == Severity::Severe))
        .unwrap();
    bb.forget_notable(severe);

    let state = bb.recent(T + 3_600_000);
    assert_eq!((state.occurrences, state.severe), (1, 0));
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

#[test]
fn recent_pressure_keeps_the_episodes_of_the_last_ten_minutes() {
    let mut bb = blackbox();
    let short_of_memory = |s: i64| {
        let mut sample = calm(T + s * 1_000);
        sample.mem_avail_bytes = GIB / 2;
        sample
    };
    for s in 0..=130 {
        bb.push_sample(short_of_memory(s));
    }
    let memory = |end_ms| PressureSpan {
        kind: EpisodeKind::MemoryHigh,
        start_ms: T,
        end_ms,
    };
    assert_eq!(bb.recent_pressure(), [memory(None)], "under way");

    feed(&mut bb, 131, 150);
    for s in 151..=300 {
        bb.push_sample(short_of_memory(s));
    }
    assert_eq!(
        bb.recent_pressure(),
        [
            memory(Some(T + 130_000)),
            PressureSpan {
                start_ms: T + 151_000,
                ..memory(None)
            },
        ],
        "the one that ended, then the one under way"
    );

    bb.drain(T + 301_000);
    bb.poll(T + 130_001 + RING_MS);
    assert_eq!(
        bb.recent_pressure(),
        [PressureSpan {
            start_ms: T + 151_000,
            ..memory(Some(T + 300_000))
        }],
        "the first left the ten minutes; the second was closed by the drain"
    );
}

#[test]
fn recent_pressure_leaves_hints_out() {
    let mut bb = blackbox();
    for s in 0..=20 {
        let mut sample = calm(T + s * 1_000);
        for core in &mut sample.cores {
            core.dpc_pct = 30.0;
        }
        bb.push_sample(sample);
    }
    feed(&mut bb, 21, 40);

    let hinted = bb
        .poll(T + 40_000)
        .into_iter()
        .any(|o| matches!(o, Output::Episode(e) if e.kind.is_hint()));
    assert!(hinted, "the hint was found");
    assert!(bb.recent_pressure().is_empty());
}

/// Three 10 ms hiccups one second apart, the first at `T + start_s` seconds.
fn stutter(bb: &mut Blackbox, start_s: i64) {
    for k in 0..3 {
        late(bb, T + (start_s + k) * 1_000, 10.0);
    }
}

#[test]
fn two_stutters_suggest_a_capture() {
    let mut bb = blackbox();
    bb.set_sensitive(true);
    feed(&mut bb, -60, 140);
    stutter(&mut bb, 1);
    stutter(&mut bb, 100);

    assert!(!bb.suggests_capture());
    let found = occurrences(bb.poll(T + 140_000));
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|o| o.kind == OccurrenceKind::Stutter));
    assert!(bb.suggests_capture());
}

#[test]
fn one_unclear_stutter_suggests_a_capture() {
    let mut bb = blackbox();
    bb.set_sensitive(true);
    feed(&mut bb, -60, 40);
    stutter(&mut bb, 1);

    let found = occurrences(bb.poll(T + 40_000));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].cause(), Cause::Unclear);
    assert!(bb.suggests_capture());
}

#[test]
fn one_explained_stutter_does_not_suggest_a_capture() {
    let mut bb = blackbox();
    bb.set_sensitive(true);
    for s in -60..=40 {
        let mut sample = calm(T + s * 1_000);
        if (-8..=2).contains(&s) {
            sample.cpu_pct = 99.0;
            sample.cores.iter_mut().for_each(|c| c.cpu_pct = 99.0);
        }
        bb.push_sample(sample);
    }
    bb.push_top(ProcessTop {
        ts_ms: T - 5_000,
        entries: vec![process(20, "builder.exe", 90.0, 1, 0.0)],
    });
    stutter(&mut bb, 1);

    let found = occurrences(bb.poll(T + 40_000));
    assert!(matches!(found[0].cause(), Cause::Cpu { .. }));
    assert!(!bb.suggests_capture());
}

#[test]
fn default_mode_never_suggests() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 140);
    stutter(&mut bb, 1);
    stutter(&mut bb, 100);
    late(&mut bb, T + 70_000, 600.0);

    assert!(!occurrences(bb.poll(T + 140_000)).is_empty());
    assert!(!bb.suggests_capture());
}

#[test]
fn turning_sensitive_mode_off_clears_the_suggestion() {
    let mut bb = blackbox();
    bb.set_sensitive(true);
    feed(&mut bb, -60, 40);
    stutter(&mut bb, 1);
    occurrences(bb.poll(T + 40_000));
    assert!(bb.suggests_capture());

    bb.set_sensitive(false);
    assert!(!bb.suggests_capture());
    bb.set_sensitive(true);
    assert!(!bb.suggests_capture());
}

const HOUR_MS: i64 = 3_600_000;

fn memory_high(ts_ms: i64) -> vestigium_core::Sample {
    let mut sample = calm(ts_ms);
    sample.mem_avail_bytes = GIB / 2;
    sample
}

#[test]
fn hang_across_a_clock_jump_is_closed_not_stretched() {
    let mut bb = blackbox();
    let running: HashSet<u32> = [7].into();
    feed(&mut bb, -60, 0);
    bb.push_hangs(T + 10_000, &[hung(7, "player.exe")], &running);
    let mut outputs = bb.clock_changed(T + 10_000, T + 12_000 + HOUR_MS);
    bb.push_hangs(T + 12_000 + HOUR_MS, &[], &running);
    outputs.extend(bb.poll(T + 12_000 + HOUR_MS + 100_000));

    for found in occurrences(outputs) {
        assert!(
            found.end_ms - found.start_ms < 60_000,
            "a hang of {} ms",
            found.end_ms - found.start_ms
        );
    }
}

#[test]
fn stutter_burst_does_not_span_a_clock_jump() {
    let mut bb = blackbox();
    bb.set_sensitive(true);
    feed(&mut bb, -60, 2);
    late(&mut bb, T + 1_000, 10.0);
    late(&mut bb, T + 2_000, 10.0);
    let mut outputs = bb.clock_changed(T + 2_000, T + 2_500 - HOUR_MS);
    late(&mut bb, T + 2_500 - HOUR_MS, 10.0);
    outputs.extend(bb.poll(T + 200_000));

    assert!(
        occurrences(outputs)
            .iter()
            .all(|o| o.kind != OccurrenceKind::Stutter)
    );
    assert!(!bb.suggests_capture());
}

#[test]
fn disk_episode_ends_at_the_jump() {
    let mut bb = blackbox();
    for s in 0..=130 {
        bb.push_sample(memory_high(T + s * 1_000));
    }
    let outputs = bb.clock_changed(T + 130_000, T + 131_000 + HOUR_MS);
    bb.push_sample(calm(T + 131_000 + HOUR_MS));

    let episodes: Vec<_> = outputs
        .iter()
        .filter_map(|o| match o {
            Output::Episode(episode) => Some(episode),
            _ => None,
        })
        .collect();
    assert_eq!(episodes.len(), 1);
    assert!(episodes[0].end_ms - episodes[0].start_ms < 200_000);
    assert!(bb.open_pressure().is_empty());
}

#[test]
fn window_after_a_jump_holds_only_new_samples() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    let jumped_to = T - 30_000;
    bb.clock_changed(T, jumped_to);
    for s in 0..=10 {
        bb.push_sample(calm(jumped_to + s * 1_000));
    }
    late(&mut bb, jumped_to + 10_000, 3_000.0);
    for s in 11..=40 {
        bb.push_sample(calm(jumped_to + s * 1_000));
    }

    let found = occurrences(bb.poll(jumped_to + 100_000));

    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .window
            .samples
            .iter()
            .all(|sample| sample.ts_ms >= jumped_to)
    );
}

#[test]
fn a_forward_jump_records_a_gap_over_the_step() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    let outputs = bb.clock_changed(T, T + HOUR_MS);

    assert!(outputs.contains(&Output::Gap(Gap {
        start_ms: T,
        end_ms: T + HOUR_MS,
        reason: GapReason::ClockChanged,
    })));
}

#[test]
fn a_backward_jump_records_a_gap_of_no_width() {
    let mut bb = blackbox();
    feed(&mut bb, -60, 0);
    let outputs = bb.clock_changed(T, T - HOUR_MS);

    assert!(outputs.contains(&Output::Gap(Gap {
        start_ms: T - HOUR_MS,
        end_ms: T - HOUR_MS,
        reason: GapReason::ClockChanged,
    })));
}
