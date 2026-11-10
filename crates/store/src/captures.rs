use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use vestigium_core::{CAPTURE_VERSION, CAPTURE_VERSION_MIN, DeepCapture};

use crate::{Store, StoreError, decode, encode};

/// When a stored deep capture ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureSummary {
    pub id: i64,
    pub start_ms: i64,
    pub end_ms: i64,
}

impl Store {
    /// Stores a finished deep capture; returns its id.
    pub fn insert_capture(&mut self, capture: &DeepCapture) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO capture (version, start_ms, end_ms, data) VALUES (?1, ?2, ?3, ?4)",
            params![
                i64::from(CAPTURE_VERSION),
                capture.start_ms,
                capture.end_ms.max(capture.start_ms),
                encode(std::slice::from_ref(capture)),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Captures that overlap `[from_ms, to_ms]`, oldest first.
    pub fn captures(&self, from_ms: i64, to_ms: i64) -> Result<Vec<CaptureSummary>, StoreError> {
        let mut query = self.conn.prepare_cached(
            "SELECT id, start_ms, end_ms FROM capture
             WHERE start_ms <= ?2 AND end_ms >= ?1 ORDER BY start_ms",
        )?;
        let rows = query.query_map(params![from_ms, to_ms], |row| {
            Ok(CaptureSummary {
                id: row.get(0)?,
                start_ms: row.get(1)?,
                end_ms: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn capture(&self, id: i64) -> Result<Option<DeepCapture>, StoreError> {
        let row: Option<(i64, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT version, data FROM capture WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(version, data)| {
            if !(i64::from(CAPTURE_VERSION_MIN)..=i64::from(CAPTURE_VERSION)).contains(&version) {
                return Err(StoreError::UnknownCode {
                    what: "capture version",
                    code: version,
                });
            }
            decode::<DeepCapture>(&data)?
                .pop()
                .ok_or(StoreError::EmptySegment)
        })
        .transpose()
    }
}
