use vestigium_core::{ProbeTick, Severity, StutterDetector, Trigger, freeze, severity};

fn tick(wall_ms: i64, lag_ms: f32) -> ProbeTick {
    ProbeTick { wall_ms, lag_ms }
}

#[test]
fn lag_below_threshold_is_no_freeze() {
    assert_eq!(freeze(&tick(10_000, 249.0)), None);
}

#[test]
fn freeze_starts_lag_before_wake() {
    assert_eq!(
        freeze(&tick(10_000, 1_500.0)),
        Some(Trigger::Freeze {
            start_ms: 8_500,
            end_ms: 10_000,
            lag_ms: 1_500.0
        })
    );
}

#[test]
fn severity_bounds() {
    assert_eq!(severity(999.0), Severity::Light);
    assert_eq!(severity(1_000.0), Severity::Medium);
    assert_eq!(severity(5_000.0), Severity::Medium);
    assert_eq!(severity(5_001.0), Severity::Severe);
}

#[test]
fn three_hiccups_in_ten_seconds_are_a_burst() {
    let mut detector = StutterDetector::new();
    assert_eq!(detector.push(&tick(1_000, 80.0)), None);
    assert_eq!(detector.push(&tick(1_016, 1.0)), None);
    assert_eq!(detector.push(&tick(5_000, 120.0)), None);
    assert_eq!(
        detector.push(&tick(9_000, 60.0)),
        Some(Trigger::Stutter {
            start_ms: 920,
            end_ms: 9_000,
            count: 3,
            worst_ms: 120.0
        })
    );
}

#[test]
fn spread_hiccups_are_not_a_burst() {
    let mut detector = StutterDetector::new();
    for wall_ms in [0, 12_500, 25_000] {
        assert_eq!(detector.push(&tick(wall_ms, 100.0)), None);
    }
}

#[test]
fn a_burst_needs_new_hiccups_to_fire_again() {
    let mut detector = StutterDetector::new();
    for wall_ms in [0, 1_000] {
        detector.push(&tick(wall_ms, 100.0));
    }
    assert!(detector.push(&tick(2_000, 100.0)).is_some());
    assert_eq!(detector.push(&tick(3_000, 100.0)), None);
    assert_eq!(detector.push(&tick(4_000, 100.0)), None);
    assert!(detector.push(&tick(5_000, 100.0)).is_some());
}

#[test]
fn freeze_sized_lag_is_not_a_hiccup() {
    let mut detector = StutterDetector::new();
    detector.push(&tick(0, 100.0));
    detector.push(&tick(1_000, 100.0));
    assert_eq!(detector.push(&tick(2_000, 250.0)), None);
    detector.reset();
    assert_eq!(detector.push(&tick(3_000, 100.0)), None);
}

#[test]
fn micro_hiccups_make_a_stutter() {
    let mut detector = StutterDetector::new();
    assert_eq!(detector.push(&tick(1_000, 8.0)), None);
    assert_eq!(detector.push(&tick(2_000, 12.0)), None);
    assert_eq!(
        detector.push(&tick(3_000, 9.5)),
        Some(Trigger::Stutter {
            start_ms: 992,
            end_ms: 3_000,
            count: 3,
            worst_ms: 12.0
        })
    );
}

#[test]
fn noise_below_8ms_is_ignored() {
    let mut detector = StutterDetector::new();
    for wall_ms in (0..10).map(|k| k * 500) {
        assert_eq!(detector.push(&tick(wall_ms, 7.9)), None);
    }
}
