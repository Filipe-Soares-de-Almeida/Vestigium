use std::path::Path;

use rusqlite::{Connection, params};

use crate::{Segment, Series, StoreError};

pub const SCHEMA_VERSION: u32 = 1;
/// SQLite page cache, in KiB; the agent only appends, so a small cache is enough.
const CACHE_KIB: i64 = 256;

/// Migration `n` moves the schema from version `n` to `n + 1`.
const MIGRATIONS: &[&str] = &["CREATE TABLE segment (
        id       INTEGER PRIMARY KEY,
        series   INTEGER NOT NULL,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL,
        data     BLOB    NOT NULL
    );
    CREATE INDEX segment_start ON segment (start_ms);"];

const _: () = assert!(MIGRATIONS.len() == SCHEMA_VERSION as usize);

/// The history database.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens or creates the database, then brings its schema up to date.
    pub fn open(path: &Path) -> Result<Store, StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_millis(2000))?;
        let found = user_version(&conn)?;
        if found > SCHEMA_VERSION {
            return Err(StoreError::NewerSchema {
                found,
                supported: SCHEMA_VERSION,
            });
        }
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", -CACHE_KIB)?;
        migrate(&mut conn, found)?;
        Ok(Store { conn })
    }

    pub fn schema_version(&self) -> Result<u32, StoreError> {
        user_version(&self.conn)
    }

    /// Page cache size in KiB.
    pub fn cache_size_kib(&self) -> Result<i64, StoreError> {
        let size: i64 = self
            .conn
            .pragma_query_value(None, "cache_size", |row| row.get(0))?;
        Ok(-size)
    }

    pub fn journal_mode(&self) -> Result<String, StoreError> {
        Ok(self
            .conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))?)
    }

    /// Writes all segments in one transaction.
    pub fn append(&mut self, segments: &[Segment]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO segment (series, start_ms, end_ms, data) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for segment in segments {
                insert.execute(params![
                    segment.series as u8,
                    segment.start_ms,
                    segment.end_ms,
                    segment.data
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Segments of `series` that overlap `[from_ms, to_ms]`, oldest first.
    pub fn segments(
        &self,
        series: Series,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<Segment>, StoreError> {
        let mut query = self.conn.prepare_cached(
            "SELECT start_ms, end_ms, data FROM segment
             WHERE series = ?1 AND start_ms <= ?3 AND end_ms >= ?2
             ORDER BY start_ms",
        )?;
        let rows = query.query_map(params![series as u8, from_ms, to_ms], |row| {
            Ok(Segment {
                series,
                start_ms: row.get(0)?,
                end_ms: row.get(1)?,
                data: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn user_version(conn: &Connection) -> Result<u32, StoreError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn migrate(conn: &mut Connection, from: u32) -> Result<(), StoreError> {
    if from == SCHEMA_VERSION {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for (version, sql) in (1..=SCHEMA_VERSION).zip(MIGRATIONS).skip(from as usize) {
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
    }
    tx.commit()?;
    Ok(())
}
