use std::collections::HashSet;

use vestigium_core::{HangOutcome, HangTracker, HungWindow, Trigger};

fn hung(pid: u32, name: &str) -> HungWindow {
    HungWindow {
        pid,
        name: name.to_string(),
    }
}

fn running(pids: &[u32]) -> HashSet<u32> {
    pids.iter().copied().collect()
}

#[test]
fn hang_starts_five_seconds_before_first_seen() {
    let mut tracker = HangTracker::new();
    assert!(
        tracker
            .push(10_000, &[hung(7, "editor.exe")], &running(&[7]))
            .is_empty()
    );
    assert!(
        tracker
            .push(12_000, &[hung(7, "editor.exe")], &running(&[7]))
            .is_empty()
    );
    assert_eq!(
        tracker.push(14_000, &[], &running(&[7])),
        vec![Trigger::AppHang {
            pid: 7,
            name: "editor.exe".to_string(),
            start_ms: 5_000,
            end_ms: 14_000,
            outcome: HangOutcome::Recovered,
        }]
    );
}

#[test]
fn recovered_and_closed_hangs_differ() {
    let mut tracker = HangTracker::new();
    tracker.push(
        0,
        &[hung(7, "editor.exe"), hung(9, "viewer.exe")],
        &running(&[7, 9]),
    );
    let outcomes: Vec<(u32, HangOutcome)> = tracker
        .push(2_000, &[], &running(&[7]))
        .into_iter()
        .filter_map(|t| match t {
            Trigger::AppHang { pid, outcome, .. } => Some((pid, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.contains(&(7, HangOutcome::Recovered)));
    assert!(outcomes.contains(&(9, HangOutcome::Closed)));
}
