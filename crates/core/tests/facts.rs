mod common;

use common::{T, event, process, quiet_window, tweak};
use vestigium_core::{
    EventMeaning, Fact, FactKind, Gap, GapReason, HangOutcome, Metric, ProcessTop, Trigger, facts,
};

fn crossed(facts: &[Fact], metric: Metric) -> usize {
    facts
        .iter()
        .filter(|f| matches!(&f.kind, FactKind::Crossed { metric: m, .. } if *m == metric))
        .count()
}

#[test]
fn facts_are_in_time_order() {
    let mut window = quiet_window();
    tweak(&mut window, -20, 5, |s| s.disks[0].latency_ms = 300.0);
    tweak(&mut window, -3, -3, |s| s.disks[0].latency_ms = 900.0);
    window.events.push(event(
        T + 5_000,
        "stornvme",
        129,
        EventMeaning::StorageReset,
    ));
    window.gaps.push(Gap {
        start_ms: T + 20_000,
        end_ms: T + 25_000,
        reason: GapReason::Paused,
    });
    let triggers = [
        Trigger::Freeze {
            start_ms: T,
            end_ms: T + 3_000,
            lag_ms: 3_000.0,
        },
        Trigger::AppHang {
            pid: 11,
            name: "player.exe".to_string(),
            start_ms: T + 1_000,
            end_ms: T + 12_000,
            outcome: HangOutcome::Closed,
        },
    ];
    let found = facts(&triggers, &window);
    assert!(found.windows(2).all(|pair| pair[0].at_ms <= pair[1].at_ms));
    assert_eq!(crossed(&found, Metric::DiskLatencyMs), 1);
    let latency = found
        .iter()
        .find(|f| matches!(f.kind, FactKind::Crossed { .. }))
        .expect("a crossing");
    assert_eq!(latency.at_ms, T - 20_000);
    assert!(matches!(latency.kind, FactKind::Crossed { value, .. } if value == 900.0));
    let kinds: Vec<&FactKind> = found.iter().map(|f| &f.kind).collect();
    assert!(kinds.contains(&&FactKind::FreezeEnded));
    assert!(kinds.contains(&&FactKind::AppClosedWhileHung {
        name: "player.exe".to_string()
    }));
    assert!(kinds.contains(&&FactKind::Gap {
        reason: GapReason::Paused
    }));
}

#[test]
fn facts_name_the_busiest_process() {
    let mut window = quiet_window();
    window.tops.push(ProcessTop {
        ts_ms: T + 5_000,
        entries: vec![process(30, "renderer.exe", 80.0, 1, 0.0)],
    });
    let found = facts(&[], &window);
    assert!(found.contains(&Fact {
        at_ms: T + 5_000,
        kind: FactKind::Busiest {
            metric: Metric::CpuPct,
            name: "renderer.exe".to_string(),
            value: 80.0,
        },
    }));
}

#[test]
fn the_crossing_names_the_disk_of_the_peak() {
    let mut window = quiet_window();
    tweak(&mut window, -20, -20, |s| s.disks[0].latency_ms = 250.0);
    tweak(&mut window, -5, -5, |s| s.disks[1].latency_ms = 800.0);
    let found = facts(&[], &window);
    let crossing = found
        .iter()
        .find(|f| {
            matches!(
                f.kind,
                FactKind::Crossed {
                    metric: Metric::DiskLatencyMs,
                    ..
                }
            )
        })
        .expect("a latency crossing");

    assert_eq!(crossing.at_ms, T - 20_000);
    let FactKind::Crossed { value, subject, .. } = &crossing.kind else {
        unreachable!();
    };
    assert_eq!(*value, 800.0);
    assert_eq!(subject.as_deref(), Some("1 D:"));
}

#[test]
fn the_queue_crossing_names_the_disk_of_the_peak() {
    let mut window = quiet_window();
    tweak(&mut window, -20, -20, |s| s.disks[0].queue_len = 8.5);
    tweak(&mut window, -5, -5, |s| s.disks[1].queue_len = 12.0);
    let found = facts(&[], &window);
    let crossing = found
        .iter()
        .find(|f| {
            matches!(
                f.kind,
                FactKind::Crossed {
                    metric: Metric::DiskQueue,
                    ..
                }
            )
        })
        .expect("a queue crossing");

    assert_eq!(crossing.at_ms, T - 20_000);
    let FactKind::Crossed { value, subject, .. } = &crossing.kind else {
        unreachable!();
    };
    assert_eq!(*value, 12.0);
    assert_eq!(subject.as_deref(), Some("1 D:"));
}

#[test]
fn a_hang_still_open_when_watching_stopped_says_so() {
    let window = quiet_window();
    let triggers = [Trigger::AppHang {
        pid: 11,
        name: "player.exe".to_string(),
        start_ms: T + 1_000,
        end_ms: T + 12_000,
        outcome: HangOutcome::StillHung,
    }];

    let found = facts(&triggers, &window);

    let kinds: Vec<&FactKind> = found.iter().map(|f| &f.kind).collect();
    assert!(kinds.contains(&&FactKind::AppStillHung {
        name: "player.exe".to_string()
    }));
    assert!(!kinds.iter().any(|k| matches!(
        k,
        FactKind::AppRecovered { .. } | FactKind::AppClosedWhileHung { .. }
    )));
}
