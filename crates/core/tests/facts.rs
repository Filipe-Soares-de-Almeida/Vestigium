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
