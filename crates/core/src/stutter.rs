use std::collections::VecDeque;
use std::ops::Range;

use crate::{FREEZE_MS, ProbeTick, Trigger};

/// Probe delays short of a freeze that a person still feels as a hitch.
pub const HICCUP_MS: Range<f32> = 50.0..FREEZE_MS;
/// Hiccups within `BURST_WINDOW_MS` that make a stutter.
pub const BURST_COUNT: usize = 3;
pub const BURST_WINDOW_MS: i64 = 10_000;

/// Finds bursts of hiccups in the sensitive-mode probe.
#[derive(Debug, Default)]
pub struct StutterDetector {
    /// Hiccups not yet part of a burst, oldest first.
    recent: VecDeque<ProbeTick>,
}

impl StutterDetector {
    pub fn new() -> StutterDetector {
        StutterDetector::default()
    }

    /// The stutter this tick completes; its hiccups never count toward another one.
    pub fn push(&mut self, tick: &ProbeTick) -> Option<Trigger> {
        if !HICCUP_MS.contains(&tick.lag_ms) {
            return None;
        }
        self.recent.push_back(*tick);
        while self
            .recent
            .front()
            .is_some_and(|first| tick.wall_ms - first.wall_ms > BURST_WINDOW_MS)
        {
            self.recent.pop_front();
        }
        if self.recent.len() < BURST_COUNT {
            return None;
        }
        let first = self.recent.front()?;
        let trigger = Trigger::Stutter {
            start_ms: first.wall_ms - first.lag_ms.round() as i64,
            end_ms: tick.wall_ms,
            count: self.recent.len() as u16,
            worst_ms: self.recent.iter().map(|t| t.lag_ms).fold(0.0, f32::max),
        };
        self.recent.clear();
        Some(trigger)
    }

    pub fn reset(&mut self) {
        self.recent.clear();
    }
}
