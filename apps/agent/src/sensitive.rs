use std::sync::{Arc, Mutex};
use std::time::Duration;

use vestigium_collect::ProbeConfig;
use vestigium_core::{DEFAULT_PROBE_PERIOD, SENSITIVE_PROBE_PERIOD};

use crate::app::lock;

/// When sensitive mode turns itself off, while it is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sensitive {
    until_ms: Option<i64>,
}

impl Sensitive {
    /// Turns it on for `length` from `now_ms`, restarting the deadline when already on.
    pub fn turn_on(&mut self, now_ms: i64, length: Duration) {
        let length_ms = i64::try_from(length.as_millis()).unwrap_or(i64::MAX);
        self.until_ms = Some(now_ms.saturating_add(length_ms));
    }

    pub fn turn_off(&mut self) {
        self.until_ms = None;
    }

    pub fn is_on(&self) -> bool {
        self.until_ms.is_some()
    }

    pub fn expired(&self, now_ms: i64) -> bool {
        self.until_ms.is_some_and(|until| now_ms >= until)
    }
}

/// Sensitive mode as the tray and the sampler share it: the deadline and the probe period it sets.
#[derive(Debug)]
pub struct SensitiveMode {
    state: Mutex<Sensitive>,
    probe: Arc<ProbeConfig>,
    length: Duration,
}

impl SensitiveMode {
    pub fn new(probe: Arc<ProbeConfig>, length: Duration) -> Arc<SensitiveMode> {
        Arc::new(SensitiveMode {
            state: Mutex::new(Sensitive::default()),
            probe,
            length,
        })
    }

    /// Turns sensitive mode on, restarting its deadline, or off.
    pub fn set(&self, on: bool, now_ms: i64) {
        let mut state = lock(&self.state);
        if on {
            state.turn_on(now_ms, self.length);
            self.probe.set(SENSITIVE_PROBE_PERIOD);
        } else {
            state.turn_off();
            self.probe.set(DEFAULT_PROBE_PERIOD);
        }
    }

    pub fn is_on(&self) -> bool {
        lock(&self.state).is_on()
    }

    /// Turns sensitive mode off once its deadline has passed; true only on the check that did it.
    pub fn expire(&self, now_ms: i64) -> bool {
        let mut state = lock(&self.state);
        if !state.expired(now_ms) {
            return false;
        }
        state.turn_off();
        self.probe.set(DEFAULT_PROBE_PERIOD);
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use vestigium_collect::ProbeConfig;
    use vestigium_core::{DEFAULT_PROBE_PERIOD, SENSITIVE_PROBE_PERIOD};

    use super::*;

    const HALF_HOUR_MS: i64 = 30 * 60 * 1000;

    #[test]
    fn sensitive_mode_expires_after_its_deadline() {
        let mut sensitive = Sensitive::default();
        assert!(!sensitive.is_on());

        sensitive.turn_on(1_000, Duration::from_millis(HALF_HOUR_MS as u64));

        assert!(sensitive.is_on());
        assert!(!sensitive.expired(HALF_HOUR_MS + 999));
        assert!(sensitive.expired(1_000 + HALF_HOUR_MS));
    }

    #[test]
    fn turning_on_again_restarts_the_deadline() {
        let mut sensitive = Sensitive::default();
        sensitive.turn_on(0, Duration::from_secs(60));
        sensitive.turn_on(50_000, Duration::from_secs(60));

        assert!(!sensitive.expired(109_999));
        assert!(sensitive.expired(110_000));
        sensitive.turn_off();
        assert!(!sensitive.is_on());
        assert!(!sensitive.expired(i64::MAX));
    }

    #[test]
    fn expiring_restores_the_default_period() {
        let probe = ProbeConfig::new(DEFAULT_PROBE_PERIOD);
        let mode = SensitiveMode::new(probe.clone(), Duration::from_secs(60));

        mode.set(true, 0);
        assert_eq!(probe.period(), SENSITIVE_PROBE_PERIOD);
        assert!(!mode.expire(59_999));
        assert!(mode.expire(60_000));
        assert!(
            !mode.expire(60_001),
            "only the first check reports the expiry"
        );

        assert!(!mode.is_on());
        assert_eq!(probe.period(), DEFAULT_PROBE_PERIOD);
    }
}
