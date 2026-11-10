use rusqlite::{Row, params};
use vestigium_core::{Applier, TWEAK_CHANGE_VERSION, TWEAKS, TweakChange, TweakChangeV1, TweakId};

use crate::{Store, StoreError, decode, encode};

/// One change to Windows as the history keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct TweakChangeRow {
    pub id: i64,
    pub applied_ms: i64,
    /// When a later change put the settings back; `None` while this one holds.
    pub undone_ms: Option<i64>,
    pub applier: Applier,
    pub change: TweakChange,
}

/// The tweak's position in the catalog, which only grows at the end.
fn tweak_code(tweak: TweakId) -> i64 {
    TWEAKS
        .iter()
        .position(|t| *t == tweak)
        .unwrap_or(usize::MAX) as i64
}

impl Store {
    /// Keeps one change; an undo also marks the change it undoes. Returns the new row's id, or the
    /// stored one when the same change was recorded at the same time before.
    pub fn record_tweak_change(
        &mut self,
        change: &TweakChange,
        applied_ms: i64,
        applier: Applier,
    ) -> Result<i64, StoreError> {
        let tx = self.conn.transaction()?;
        let stored = tx.execute(
            "INSERT OR IGNORE INTO tweak_change (version, tweak, subject, applied_ms, undone_ms,
                 undo_of, elevated, data)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7)",
            params![
                i64::from(TWEAK_CHANGE_VERSION),
                tweak_code(change.tweak),
                change.subject,
                applied_ms,
                change.undo_of,
                applier == Applier::Elevated,
                encode(std::slice::from_ref(change))
            ],
        )?;
        if stored == 0 {
            return Ok(tx.query_row(
                "SELECT id FROM tweak_change
                 WHERE applied_ms = ?1 AND tweak = ?2 AND COALESCE(subject, '') = COALESCE(?3, '')
                   AND COALESCE(undo_of, 0) = COALESCE(?4, 0)",
                params![
                    applied_ms,
                    tweak_code(change.tweak),
                    change.subject,
                    change.undo_of
                ],
                |row| row.get(0),
            )?);
        }
        let id = tx.last_insert_rowid();
        if let Some(undone) = change.undo_of.filter(|_| change.succeeded()) {
            tx.execute(
                "UPDATE tweak_change SET undone_ms = ?2 WHERE id = ?1 AND undone_ms IS NULL",
                params![undone, applied_ms],
            )?;
        }
        tx.commit()?;
        Ok(id)
    }

    /// Changes made in `[from_ms, to_ms]`, newest first, undos included.
    pub fn tweak_changes(
        &self,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<TweakChangeRow>, StoreError> {
        self.tweak_rows(
            "WHERE applied_ms BETWEEN ?1 AND ?2",
            params![from_ms, to_ms],
        )
    }

    pub fn tweak_change(&self, id: i64) -> Result<Option<TweakChangeRow>, StoreError> {
        Ok(self.tweak_rows("WHERE id = ?1", params![id])?.pop())
    }

    /// Changes still in place, whatever their age: each can still be undone.
    pub fn tweaks_in_place(&self) -> Result<Vec<TweakChangeRow>, StoreError> {
        self.tweak_rows("WHERE undone_ms IS NULL AND undo_of IS NULL", params![])
    }

    fn tweak_rows(
        &self,
        filter: &str,
        args: impl rusqlite::Params,
    ) -> Result<Vec<TweakChangeRow>, StoreError> {
        let mut query = self.conn.prepare(&format!(
            "SELECT id, applied_ms, undone_ms, elevated, version, data FROM tweak_change {filter}
             ORDER BY applied_ms DESC, id DESC"
        ))?;
        let rows = query.query_map(args, |row: &Row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, applied_ms, undone_ms, elevated, version, data) = row?;
            let change = if version == i64::from(TWEAK_CHANGE_VERSION) {
                decode::<TweakChange>(&data)?.pop()
            } else if version == 1 {
                decode::<TweakChangeV1>(&data)?.pop().map(TweakChange::from)
            } else {
                return Err(StoreError::UnknownCode {
                    what: "tweak change version",
                    code: version,
                });
            };
            Ok(TweakChangeRow {
                id,
                applied_ms,
                undone_ms,
                applier: if elevated {
                    Applier::Elevated
                } else {
                    Applier::User
                },
                change: change.ok_or(StoreError::EmptySegment)?,
            })
        })
        .collect()
    }
}
