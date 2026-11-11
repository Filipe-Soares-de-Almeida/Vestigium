//! Tells a wall-clock step apart from the unbiased clock that paces the sampler, and moves a
//! deadline between the wall clock the window speaks and the steady clock the agent counts on.

/// A step of the wall clock beyond the unbiased clock bigger than this is a clock change.
pub const JUMP_THRESHOLD_MS: f64 = 2_000.0;

/// A wall-clock step: where the wall clock would have been, and where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockJump {
    pub from_wall_ms: i64,
    pub to_wall_ms: i64,
}

/// Compares each wall-clock reading with the unbiased clock read beside it.
pub struct ClockWatch {
    /// Wall minus unbiased milliseconds at the previous reading.
    offset_ms: f64,
}

impl ClockWatch {
    pub fn new(wall_ms: i64, unbiased_ms: f64) -> ClockWatch {
        ClockWatch {
            offset_ms: wall_ms as f64 - unbiased_ms,
        }
    }

    /// Reports a jump when the wall clock moved from the previous reading by more than
    /// the threshold beyond the unbiased clock; the reading is the new reference either way.
    pub fn observe(&mut self, wall_ms: i64, unbiased_ms: f64) -> Option<ClockJump> {
        let expected = unbiased_ms + self.offset_ms;
        self.offset_ms = wall_ms as f64 - unbiased_ms;
        ((wall_ms as f64 - expected).abs() > JUMP_THRESHOLD_MS).then_some(ClockJump {
            from_wall_ms: expected.round() as i64,
            to_wall_ms: wall_ms,
        })
    }

    /// Takes the readings as the reference without judging them, after a pause or a suspend.
    pub fn rebase(&mut self, wall_ms: i64, unbiased_ms: f64) {
        self.offset_ms = wall_ms as f64 - unbiased_ms;
    }
}

/// The steady-clock instant that matches a wall-clock instant, given both clocks read together.
pub fn wall_to_steady(wall_instant_ms: i64, now_wall_ms: i64, now_steady_ms: i64) -> i64 {
    now_steady_ms.saturating_add(wall_instant_ms.saturating_sub(now_wall_ms))
}

/// The wall-clock instant that matches a steady-clock instant, given both clocks read together.
pub fn steady_to_wall(steady_instant_ms: i64, now_wall_ms: i64, now_steady_ms: i64) -> i64 {
    now_wall_ms.saturating_add(steady_instant_ms.saturating_sub(now_steady_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deadline_keeps_its_remaining_time_across_the_two_clocks() {
        let (wall, steady) = (1_800_000_000_000, 7_000_000);
        let steady_end = wall_to_steady(wall + 1_800_000, wall, steady);
        assert_eq!(steady_end, steady + 1_800_000);
        assert_eq!(steady_to_wall(steady_end, wall, steady), wall + 1_800_000);
    }

    #[test]
    fn a_wall_clock_step_moves_the_shown_end_but_not_the_remaining_time() {
        let (wall, steady) = (1_800_000_000_000, 7_000_000);
        let steady_end = wall_to_steady(wall + 1_800_000, wall, steady);

        // A minute later the wall clock goes back an hour; the steady clock does not notice.
        let (wall_after, steady_after) = (wall + 60_000 - 3_600_000, steady + 60_000);
        let shown = steady_to_wall(steady_end, wall_after, steady_after);

        assert_eq!(shown - wall_after, 1_740_000, "29 minutes are still left");
        assert_eq!(steady_end - steady_after, 1_740_000);
    }

    const START_WALL: i64 = 1_800_000_000_000;

    fn watch() -> ClockWatch {
        ClockWatch::new(START_WALL, 5_000.0)
    }

    #[test]
    fn jump_forward_is_reported() {
        let mut watch = watch();
        let jump = watch
            .observe(START_WALL + 1_000 + 3_600_000, 6_000.0)
            .unwrap();
        assert_eq!(jump.from_wall_ms, START_WALL + 1_000);
        assert_eq!(jump.to_wall_ms, START_WALL + 1_000 + 3_600_000);
    }

    #[test]
    fn jump_back_is_reported() {
        let mut watch = watch();
        let jump = watch
            .observe(START_WALL + 1_000 - 3_600_000, 6_000.0)
            .unwrap();
        assert_eq!(jump.from_wall_ms, START_WALL + 1_000);
        assert!(jump.to_wall_ms < jump.from_wall_ms);
    }

    #[test]
    fn jitter_under_two_seconds_is_ignored() {
        let mut watch = watch();
        assert_eq!(watch.observe(START_WALL + 1_000 + 1_500, 6_000.0), None);
        assert_eq!(watch.observe(START_WALL + 2_000 + 1_000, 7_000.0), None);
        assert_eq!(watch.observe(START_WALL + 3_000 - 500, 8_000.0), None);
    }

    #[test]
    fn rebase_after_a_suspend_reports_nothing() {
        let mut watch = watch();
        watch.rebase(START_WALL + 8 * 3_600_000, 6_000.0);
        assert_eq!(
            watch.observe(START_WALL + 8 * 3_600_000 + 1_000, 7_000.0),
            None
        );
    }

    #[test]
    fn a_second_tick_after_a_jump_reports_nothing() {
        let mut watch = watch();
        assert!(
            watch
                .observe(START_WALL + 1_000 + 7_200_000, 6_000.0)
                .is_some()
        );
        assert_eq!(watch.observe(START_WALL + 2_000 + 7_200_000, 7_000.0), None);
    }

    #[test]
    fn a_step_of_local_time_without_a_change_in_epoch_time_reports_nothing() {
        // Daylight saving moves the local reading, not the epoch milliseconds read here.
        let mut watch = watch();
        assert_eq!(watch.observe(START_WALL + 1_000, 6_000.0), None);
        assert_eq!(watch.observe(START_WALL + 2_000, 7_000.0), None);
    }

    #[test]
    fn a_slow_drift_is_never_a_jump() {
        let mut watch = watch();
        for tick in 1..=600 {
            let unbiased = 5_000.0 + tick as f64 * 1_000.0;
            let wall = START_WALL + tick * 1_000 + tick * 10;
            assert_eq!(watch.observe(wall, unbiased), None);
        }
    }
}
