use serde::{Deserialize, Serialize};

/// One tick of the stall probe.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeTick {
    /// Wall clock when the tick woke up, Unix epoch milliseconds.
    pub wall_ms: i64,
    /// How late it woke, on a clock that leaves out sleep and hibernation.
    pub lag_ms: f32,
}
