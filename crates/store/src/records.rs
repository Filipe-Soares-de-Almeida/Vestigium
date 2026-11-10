use rusqlite::{OptionalExtension, Row, params};
use vestigium_core::{
    Cause, Channel, ColorTheme, DetailLevel, Episode, EpisodeKind, EventMeaning, Gap, GapReason,
    LanguagePref, MachineInfo, Occurrence, OccurrenceKind, OccurrenceSummary, Retention, Settings,
    Severity, ThemePref, WinEvent,
};

use crate::{Store, StoreError, decode, encode};

/// Layout of the `Occurrence` and `Episode` BLOBs written today.
const RECORD_VERSION: i64 = 1;

const KINDS: [OccurrenceKind; 5] = [
    OccurrenceKind::Freeze,
    OccurrenceKind::Stutter,
    OccurrenceKind::AppHang,
    OccurrenceKind::Reported,
    OccurrenceKind::PostMortem,
];
const SEVERITIES: [Severity; 3] = [Severity::Light, Severity::Medium, Severity::Severe];
const CHANNELS: [Channel; 2] = [Channel::System, Channel::Application];
const REASONS: [GapReason; 5] = [
    GapReason::Paused,
    GapReason::Suspended,
    GapReason::Stopped,
    GapReason::Crashed,
    GapReason::ClockChanged,
];
/// Stored meaning codes are positions in this list plus one; new meanings go at the end.
const MEANINGS: [EventMeaning; 16] = [
    EventMeaning::StorageReset,
    EventMeaning::DiskRetry,
    EventMeaning::DiskBadBlock,
    EventMeaning::PagingError,
    EventMeaning::DisplayReset,
    EventMeaning::HardwareError,
    EventMeaning::PowerLoss,
    EventMeaning::UnexpectedShutdown,
    EventMeaning::BugCheck,
    EventMeaning::CpuFirmwareLimit,
    EventMeaning::LowVirtualMemory,
    EventMeaning::AppHang,
    EventMeaning::AppCrash,
    EventMeaning::ServiceCrash,
    EventMeaning::NoCrashDump,
    EventMeaning::MemoryDiagnostic,
];

/// The stored code of `value`: its position in `list` plus one; a value missing from it is refused.
fn code<T: PartialEq + std::fmt::Debug>(
    list: &[T],
    value: &T,
    what: &'static str,
) -> Result<i64, StoreError> {
    list.iter()
        .position(|v| v == value)
        .map(|i| i as i64 + 1)
        .ok_or_else(|| StoreError::UnwritableValue {
            what,
            value: format!("{value:?}"),
        })
}

fn from_code<T: Copy>(list: &[T], code: i64, what: &'static str) -> Result<T, StoreError> {
    usize::try_from(code - 1)
        .ok()
        .and_then(|i| list.get(i).copied())
        .ok_or(StoreError::UnknownCode { what, code })
}

pub(crate) fn channel_code(channel: Channel) -> Result<i64, StoreError> {
    code(&CHANNELS, &channel, "channel")
}

pub(crate) fn meaning_code(meaning: EventMeaning) -> Result<i64, StoreError> {
    code(&MEANINGS, &meaning, "event meaning")
}

pub(crate) fn gap_code(reason: GapReason) -> Result<i64, StoreError> {
    code(&REASONS, &reason, "gap reason")
}

fn episode_code(kind: &EpisodeKind) -> i64 {
    match kind {
        EpisodeKind::MemoryHigh => 1,
        EpisodeKind::DiskBusy { .. } => 2,
        EpisodeKind::CpuSaturated => 3,
        EpisodeKind::DpcIsrHint { .. } => 4,
        EpisodeKind::DiskLatencyHint { .. } => 5,
    }
}

const SETTING_RETENTION: &str = "retention";
const SETTING_LANGUAGE: &str = "language";
const SETTING_THEME: &str = "theme";
const SETTING_COLOR_THEME: &str = "color_theme";
const SETTING_DETAIL_LEVEL: &str = "detail_level";

impl Store {
    /// Stores a finished occurrence on its own, right away; returns its id.
    pub fn insert_occurrence(&mut self, occurrence: &Occurrence) -> Result<i64, StoreError> {
        let kind = code(&KINDS, &occurrence.kind, "occurrence kind")?;
        let severity = code(&SEVERITIES, &occurrence.severity, "severity")?;
        self.conn.execute(
            "INSERT INTO occurrence (version, kind, start_ms, end_ms, severity, cause, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                RECORD_VERSION,
                kind,
                occurrence.start_ms,
                occurrence.end_ms,
                severity,
                postcard::to_stdvec(&occurrence.cause())?,
                encode(std::slice::from_ref(occurrence)),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_episode(&mut self, episode: &Episode) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO episode (version, kind, start_ms, end_ms, data)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                RECORD_VERSION,
                episode_code(&episode.kind),
                episode.start_ms,
                episode.end_ms,
                encode(std::slice::from_ref(episode)),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn set_machine(&mut self, machine: &MachineInfo) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO machine (id, data) VALUES (1, ?1)
             ON CONFLICT (id) DO UPDATE SET data = excluded.data",
            params![serde_json::to_string(machine)?],
        )?;
        Ok(())
    }

    /// Stores settings that pass validation; nothing is written otherwise.
    pub fn set_settings(&mut self, settings: &Settings) -> Result<(), StoreError> {
        settings.validate()?;
        let tx = self.conn.transaction()?;
        for (key, value) in [
            (
                SETTING_RETENTION,
                serde_json::to_string(&settings.retention)?,
            ),
            (SETTING_LANGUAGE, serde_json::to_string(&settings.language)?),
            (SETTING_THEME, serde_json::to_string(&settings.theme)?),
            (
                SETTING_COLOR_THEME,
                serde_json::to_string(&settings.color_theme)?,
            ),
            (
                SETTING_DETAIL_LEVEL,
                serde_json::to_string(&settings.detail_level)?,
            ),
        ] {
            tx.execute(
                "INSERT INTO setting (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Occurrences that overlap `[from_ms, to_ms]`, oldest first.
    pub fn occurrences(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<OccurrenceSummary>, StoreError> {
        let mut query = self.conn.prepare_cached(
            "SELECT id, kind, start_ms, end_ms, severity, cause FROM occurrence
             WHERE start_ms <= ?2 AND end_ms >= ?1 ORDER BY start_ms",
        )?;
        let rows = query.query_map(params![from_ms, to_ms], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })?;
        let mut found = Vec::new();
        for row in rows {
            let (id, kind, start_ms, end_ms, severity, cause) = row?;
            found.push(OccurrenceSummary {
                id,
                kind: from_code(&KINDS, kind, "occurrence kind")?,
                start_ms,
                end_ms,
                severity: from_code(&SEVERITIES, severity, "severity")?,
                cause: postcard::from_bytes::<Cause>(&cause)?,
            });
        }
        Ok(found)
    }

    pub fn occurrence(&self, id: i64) -> Result<Option<Occurrence>, StoreError> {
        let row: Option<(i64, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT version, data FROM occurrence WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(version, data)| one::<Occurrence>(version, &data))
            .transpose()
    }

    /// Episodes that overlap `[from_ms, to_ms]`, oldest first.
    pub fn episodes(&self, from_ms: i64, to_ms: i64) -> Result<Vec<Episode>, StoreError> {
        let mut query = self.conn.prepare_cached(
            "SELECT version, data FROM episode
             WHERE start_ms <= ?2 AND end_ms >= ?1 ORDER BY start_ms",
        )?;
        let rows = query.query_map(params![from_ms, to_ms], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;
        let mut found = Vec::new();
        for row in rows {
            let (version, data) = row?;
            found.push(one::<Episode>(version, &data)?);
        }
        Ok(found)
    }

    /// Curated events logged in `[from_ms, to_ms]`, oldest first.
    pub fn events(&self, from_ms: i64, to_ms: i64) -> Result<Vec<WinEvent>, StoreError> {
        let mut query = self.conn.prepare_cached(
            "SELECT ts_ms, channel, provider, event_id, record_id, level, meaning, data_xml
             FROM event WHERE ts_ms BETWEEN ?1 AND ?2 ORDER BY ts_ms, id",
        )?;
        let rows = query.query_map(params![from_ms, to_ms], event_row)?;
        let mut found = Vec::new();
        for row in rows {
            let (event, channel, meaning) = row?;
            found.push(WinEvent {
                channel: from_code(&CHANNELS, channel, "channel")?,
                meaning: from_code(&MEANINGS, meaning, "event meaning")?,
                ..event
            });
        }
        Ok(found)
    }

    /// Gaps that overlap `[from_ms, to_ms]`, oldest first.
    pub fn gaps(&self, from_ms: i64, to_ms: i64) -> Result<Vec<Gap>, StoreError> {
        let mut query = self.conn.prepare_cached(
            "SELECT start_ms, end_ms, reason FROM gap
             WHERE start_ms <= ?2 AND end_ms >= ?1 ORDER BY start_ms",
        )?;
        let rows = query.query_map(params![from_ms, to_ms], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        let mut found = Vec::new();
        for row in rows {
            let (start_ms, end_ms, reason) = row?;
            found.push(Gap {
                start_ms,
                end_ms,
                reason: from_code(&REASONS, reason, "gap reason")?,
            });
        }
        Ok(found)
    }

    pub fn bookmark_xml(&self) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT xml FROM event_bookmark WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?)
    }

    /// Stored settings; a key that is missing or unreadable takes its default.
    pub fn settings(&self) -> Result<Settings, StoreError> {
        let mut query = self.conn.prepare_cached("SELECT key, value FROM setting")?;
        let rows = query.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut settings = Settings::default();
        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                SETTING_RETENTION => {
                    if let Ok(retention) = serde_json::from_str::<Retention>(&value) {
                        settings.retention = retention;
                    }
                }
                SETTING_LANGUAGE => {
                    if let Ok(language) = serde_json::from_str::<LanguagePref>(&value) {
                        settings.language = language;
                    }
                }
                SETTING_THEME => {
                    if let Ok(theme) = serde_json::from_str::<ThemePref>(&value) {
                        settings.theme = theme;
                    }
                }
                SETTING_COLOR_THEME => {
                    if let Ok(color_theme) = serde_json::from_str::<ColorTheme>(&value) {
                        settings.color_theme = color_theme;
                    }
                }
                SETTING_DETAIL_LEVEL => {
                    if let Ok(detail_level) = serde_json::from_str::<DetailLevel>(&value) {
                        settings.detail_level = detail_level;
                    }
                }
                _ => {}
            }
        }
        if settings.validate().is_err() {
            settings.retention = Retention::default();
        }
        Ok(settings)
    }

    pub fn machine(&self) -> Result<Option<MachineInfo>, StoreError> {
        let data: Option<String> = self
            .conn
            .query_row("SELECT data FROM machine WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(data.map(|d| serde_json::from_str(&d)).transpose()?)
    }
}

type EventRow = (WinEvent, i64, i64);

fn event_row(row: &Row<'_>) -> rusqlite::Result<EventRow> {
    let event = WinEvent {
        ts_ms: row.get(0)?,
        channel: Channel::System,
        provider: row.get(2)?,
        id: row.get(3)?,
        record_id: row.get::<_, i64>(4)?.max(0) as u64,
        level: row.get(5)?,
        meaning: EventMeaning::StorageReset,
        data_xml: row.get(7)?,
    };
    Ok((event, row.get(1)?, row.get(6)?))
}

/// Decodes a record BLOB of the given layout version.
fn one<T: serde::de::DeserializeOwned>(version: i64, data: &[u8]) -> Result<T, StoreError> {
    if version != RECORD_VERSION {
        return Err(StoreError::UnknownCode {
            what: "record version",
            code: version,
        });
    }
    decode::<T>(data)?.pop().ok_or(StoreError::EmptySegment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_missing_from_its_table_is_refused() {
        const PARTIAL: [Severity; 2] = [Severity::Light, Severity::Medium];

        assert_eq!(code(&PARTIAL, &Severity::Medium, "severity").unwrap(), 2);
        assert!(matches!(
            code(&PARTIAL, &Severity::Severe, "severity"),
            Err(StoreError::UnwritableValue { what: "severity", value }) if value == "Severe"
        ));
    }
}
