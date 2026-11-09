use crate::{ProbeTick, Trigger};

/// A probe tick at least this late means the whole machine stopped.
pub const FREEZE_MS: f32 = 250.0;

/// The freeze a late tick reveals; it began when the tick should have woken.
pub fn freeze(tick: &ProbeTick) -> Option<Trigger> {
    (tick.lag_ms >= FREEZE_MS).then(|| Trigger::Freeze {
        start_ms: tick.wall_ms - tick.lag_ms.round() as i64,
        end_ms: tick.wall_ms,
        lag_ms: tick.lag_ms,
    })
}
