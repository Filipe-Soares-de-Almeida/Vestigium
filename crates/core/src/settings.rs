use std::fmt;

use serde::{Deserialize, Serialize};

/// How long each kind of history is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retention {
    /// Per-second samples and process rankings.
    pub samples_hours: u32,
    /// One-minute summaries.
    pub minutes_days: u32,
    /// Occurrences, episodes, events and gaps.
    pub records_days: u32,
}

impl Default for Retention {
    fn default() -> Retention {
        Retention {
            samples_hours: 48,
            minutes_days: 90,
            records_days: 365,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguagePref {
    #[default]
    System,
    En,
    PtBr,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

/// What a person can change; starting with Windows lives in the registry, not here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub retention: Retention,
    pub language: LanguagePref,
    pub theme: ThemePref,
}

/// A retention value outside what the watcher accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingError {
    SamplesHours(u32),
    MinutesDays(u32),
    RecordsDays(u32),
}

impl fmt::Display for SettingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingError::SamplesHours(v) => write!(f, "samples kept {v} h, not 24 to 168"),
            SettingError::MinutesDays(v) => write!(f, "minutes kept {v} days, not 30 to 365"),
            SettingError::RecordsDays(v) => write!(f, "records kept {v} days, not 90 to 730"),
        }
    }
}

impl std::error::Error for SettingError {}

impl Settings {
    pub fn validate(&self) -> Result<(), SettingError> {
        let r = self.retention;
        if !(24..=168).contains(&r.samples_hours) {
            return Err(SettingError::SamplesHours(r.samples_hours));
        }
        if !(30..=365).contains(&r.minutes_days) {
            return Err(SettingError::MinutesDays(r.minutes_days));
        }
        if !(90..=730).contains(&r.records_days) {
            return Err(SettingError::RecordsDays(r.records_days));
        }
        Ok(())
    }
}
