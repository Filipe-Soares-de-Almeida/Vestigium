use rusqlite::{OptionalExtension, Row, params};
use vestigium_core::{DUMP_VERSION, DumpRecord};

use crate::{Store, StoreError, decode, encode};

/// A dump Windows kept, with when the watcher learned of it, read it and saw it gone.
#[derive(Clone, Debug, PartialEq)]
pub struct DumpRow {
    pub id: i64,
    pub found_ms: i64,
    pub analyzed_ms: Option<i64>,
    /// `None` while the file is still there.
    pub deleted_ms: Option<i64>,
    pub record: DumpRecord,
}

impl Store {
    /// Adds dumps not seen before and refreshes known ones, matched by path and file time; a
    /// record with no analysis never replaces one that has it. Returns the ids touched.
    pub fn upsert_dumps(
        &mut self,
        records: &[DumpRecord],
        now_ms: i64,
    ) -> Result<Vec<i64>, StoreError> {
        let tx = self.conn.transaction()?;
        let mut ids = Vec::with_capacity(records.len());
        for record in records {
            let known: Option<(i64, Vec<u8>)> = tx
                .query_row(
                    "SELECT id, data FROM dump WHERE path = ?1 AND file_ms = ?2",
                    params![record.path, record.file_ms],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let analyzed = record.analysis.as_ref().map(|_| now_ms);
            let id = match known {
                None => {
                    tx.execute(
                        "INSERT INTO dump (version, path, file_ms, crash_ms, size_bytes, found_ms,
                             analyzed_ms, deleted_ms, data)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
                        params![
                            i64::from(DUMP_VERSION),
                            record.path,
                            record.file_ms,
                            record.crash_ms,
                            record.size_bytes as i64,
                            now_ms,
                            analyzed,
                            encode(std::slice::from_ref(record))
                        ],
                    )?;
                    tx.last_insert_rowid()
                }
                Some((id, data)) => {
                    let stored: Option<DumpRecord> = decode::<DumpRecord>(&data)?.pop();
                    let keeps_analysis = record.analysis.is_none()
                        && stored.as_ref().is_some_and(|s| s.analysis.is_some());
                    if keeps_analysis {
                        tx.execute(
                            "UPDATE dump SET size_bytes = ?2, deleted_ms = NULL WHERE id = ?1",
                            params![id, record.size_bytes as i64],
                        )?;
                    } else {
                        tx.execute(
                            "UPDATE dump SET version = ?2, crash_ms = ?3, size_bytes = ?4,
                                 analyzed_ms = COALESCE(?5, analyzed_ms), deleted_ms = NULL,
                                 data = ?6
                             WHERE id = ?1",
                            params![
                                id,
                                i64::from(DUMP_VERSION),
                                record.crash_ms,
                                record.size_bytes as i64,
                                analyzed,
                                encode(std::slice::from_ref(record))
                            ],
                        )?;
                    }
                    id
                }
            };
            ids.push(id);
        }
        tx.commit()?;
        Ok(ids)
    }

    /// Marks the dumps with these ids as gone from disk; their analysis stays.
    pub fn mark_dumps_deleted(&mut self, ids: &[i64], now_ms: i64) -> Result<usize, StoreError> {
        let tx = self.conn.transaction()?;
        let mut marked = 0;
        for id in ids {
            marked += tx.execute(
                "UPDATE dump SET deleted_ms = ?2 WHERE id = ?1 AND deleted_ms IS NULL",
                params![id, now_ms],
            )?;
        }
        tx.commit()?;
        Ok(marked)
    }

    /// Dumps whose crash falls in `[from_ms, to_ms]`, newest first, deleted ones included.
    pub fn dumps(&self, from_ms: i64, to_ms: i64) -> Result<Vec<DumpRow>, StoreError> {
        self.dump_rows("WHERE crash_ms BETWEEN ?1 AND ?2", params![from_ms, to_ms])
    }

    /// Dumps still on disk, newest first.
    pub fn present_dumps(&self) -> Result<Vec<DumpRow>, StoreError> {
        self.dump_rows("WHERE deleted_ms IS NULL", params![])
    }

    /// Bytes the dumps still on disk take.
    pub fn dump_space(&self) -> Result<u64, StoreError> {
        let total: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM dump WHERE deleted_ms IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(total.max(0) as u64)
    }

    fn dump_rows(
        &self,
        filter: &str,
        args: impl rusqlite::Params,
    ) -> Result<Vec<DumpRow>, StoreError> {
        let mut query = self.conn.prepare(&format!(
            "SELECT id, found_ms, analyzed_ms, deleted_ms, version, data FROM dump {filter}
             ORDER BY crash_ms DESC, id DESC"
        ))?;
        let rows = query.query_map(args, |row: &Row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, found_ms, analyzed_ms, deleted_ms, version, data) = row?;
            if version != i64::from(DUMP_VERSION) {
                return Err(StoreError::UnknownCode {
                    what: "dump version",
                    code: version,
                });
            }
            Ok(DumpRow {
                id,
                found_ms,
                analyzed_ms,
                deleted_ms,
                record: decode::<DumpRecord>(&data)?
                    .pop()
                    .ok_or(StoreError::EmptySegment)?,
            })
        })
        .collect()
    }
}
