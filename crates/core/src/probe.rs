use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Stall-probe period in the default mode.
pub const DEFAULT_PROBE_PERIOD: Duration = Duration::from_millis(100);
/// Stall-probe period in sensitive mode, about one frame at 60 Hz.
pub const SENSITIVE_PROBE_PERIOD: Duration = Duration::from_millis(16);
/// How long sensitive mode stays on before it turns itself off.
pub const SENSITIVE_FOR: Duration = Duration::from_secs(30 * 60);

/// One tick of the stall probe.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeTick {
    /// Wall clock when the tick woke up, Unix epoch milliseconds.
    pub wall_ms: i64,
    /// How late it woke, on a clock that leaves out sleep and hibernation.
    pub lag_ms: f32,
}
