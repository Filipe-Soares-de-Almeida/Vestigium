use std::path::Path;

use rusqlite::{Connection, OpenFlags, params};
use vestigium_core::{Gap, WinEvent};

use crate::records::{channel_code, gap_code, meaning_code};
use crate::{Segment, Series, StoreError};

pub const SCHEMA_VERSION: u32 = 8;
/// SQLite page cache, in KiB; the agent only appends, so a small cache is enough.
const CACHE_KIB: i64 = 256;

/// Migration `n` moves the schema from version `n` to `n + 1`.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE segment (
        id       INTEGER PRIMARY KEY,
        series   INTEGER NOT NULL,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL,
        data     BLOB    NOT NULL
    );
    CREATE INDEX segment_start ON segment (start_ms);",
    "CREATE TABLE occurrence (
        id       INTEGER PRIMARY KEY,
        version  INTEGER NOT NULL,
        kind     INTEGER NOT NULL,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL CHECK (end_ms >= start_ms),
        severity INTEGER NOT NULL,
        cause    BLOB    NOT NULL,
        data     BLOB    NOT NULL
    );
    CREATE INDEX occurrence_start ON occurrence (start_ms);
    CREATE TABLE episode (
        id       INTEGER PRIMARY KEY,
        version  INTEGER NOT NULL,
        kind     INTEGER NOT NULL,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL CHECK (end_ms >= start_ms),
        data     BLOB    NOT NULL
    );
    CREATE INDEX episode_start ON episode (start_ms);
    CREATE TABLE event (
        id        INTEGER PRIMARY KEY,
        ts_ms     INTEGER NOT NULL,
        channel   INTEGER NOT NULL,
        provider  TEXT    NOT NULL,
        event_id  INTEGER NOT NULL,
        record_id INTEGER NOT NULL,
        level     INTEGER NOT NULL,
        meaning   INTEGER NOT NULL,
        data_xml  TEXT    NOT NULL,
        UNIQUE (channel, record_id)
    );
    CREATE INDEX event_ts ON event (ts_ms);
    CREATE TABLE event_bookmark (
        id  INTEGER PRIMARY KEY CHECK (id = 1),
        xml TEXT NOT NULL
    );
    CREATE TABLE gap (
        id       INTEGER PRIMARY KEY,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL CHECK (end_ms >= start_ms),
        reason   INTEGER NOT NULL
    );
    CREATE INDEX gap_start ON gap (start_ms);
    CREATE TABLE setting (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE machine (
        id   INTEGER PRIMARY KEY CHECK (id = 1),
        data TEXT NOT NULL
    );",
    "CREATE TABLE capture (
        id       INTEGER PRIMARY KEY,
        version  INTEGER NOT NULL,
        start_ms INTEGER NOT NULL,
        end_ms   INTEGER NOT NULL CHECK (end_ms >= start_ms),
        data     BLOB    NOT NULL
    );
    CREATE INDEX capture_start ON capture (start_ms);",
    "CREATE TABLE finding (
        id         INTEGER PRIMARY KEY,
        version    INTEGER NOT NULL,
        opened_ms  INTEGER NOT NULL,
        updated_ms INTEGER NOT NULL,
        closed_ms  INTEGER CHECK (closed_ms IS NULL OR closed_ms >= opened_ms),
        data       BLOB    NOT NULL
    );
    CREATE INDEX finding_opened ON finding (opened_ms);
    CREATE TABLE scan (
        id      INTEGER PRIMARY KEY,
        version INTEGER NOT NULL,
        ts_ms   INTEGER NOT NULL,
        data    BLOB    NOT NULL
    );
    CREATE INDEX scan_ts ON scan (ts_ms);",
    "CREATE TABLE dump (
        id          INTEGER PRIMARY KEY,
        version     INTEGER NOT NULL,
        path        TEXT    NOT NULL,
        file_ms     INTEGER NOT NULL,
        crash_ms    INTEGER NOT NULL,
        size_bytes  INTEGER NOT NULL,
        found_ms    INTEGER NOT NULL,
        analyzed_ms INTEGER,
        deleted_ms  INTEGER,
        data        BLOB    NOT NULL,
        UNIQUE (path, file_ms)
    );
    CREATE INDEX dump_crash ON dump (crash_ms);",
    "CREATE TABLE tweak_change (
        id         INTEGER PRIMARY KEY,
        version    INTEGER NOT NULL,
        tweak      INTEGER NOT NULL,
        subject    TEXT,
        applied_ms INTEGER NOT NULL,
        undone_ms  INTEGER,
        undo_of    INTEGER,
        elevated   INTEGER NOT NULL,
        data       BLOB    NOT NULL
    );
    CREATE INDEX tweak_change_applied ON tweak_change (applied_ms);",
    "CREATE TABLE test_run (
        id         INTEGER PRIMARY KEY,
        version    INTEGER NOT NULL,
        kind       INTEGER NOT NULL,
        subject    TEXT    NOT NULL,
        started_ms INTEGER NOT NULL,
        ended_ms   INTEGER NOT NULL,
        outcome    INTEGER NOT NULL,
        data       BLOB    NOT NULL,
        UNIQUE (kind, subject, started_ms)
    );
    CREATE INDEX test_run_ended ON test_run (ended_ms);",
    // A finding keeps what it measured when it opened, to show how it changed since.
    "ALTER TABLE finding ADD COLUMN first_version INTEGER;
    ALTER TABLE finding ADD COLUMN first_data BLOB;
    UPDATE finding SET first_version = version, first_data = data WHERE updated_ms = opened_ms;",
];

/// What the agent writes in one transaction, every batch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batch {
    pub segments: Vec<Segment>,
    pub events: Vec<WinEvent>,
    /// Where the event subscription resumes; written with the events it covers.
    pub bookmark_xml: Option<String>,
    pub gaps: Vec<Gap>,
}

impl Batch {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
            && self.events.is_empty()
            && self.bookmark_xml.is_none()
            && self.gaps.is_empty()
    }
}

const _: () = assert!(MIGRATIONS.len() == SCHEMA_VERSION as usize);

/// The history database.
#[derive(Debug)]
pub struct Store {
    pub(crate) conn: Connection,
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

    /// Opens an existing database without writing to it, as the UI does while the agent runs.
    pub fn open_read_only(path: &Path) -> Result<Store, StoreError> {
        if !path.is_file() {
            return Err(StoreError::NotFound);
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        conn.busy_timeout(std::time::Duration::from_millis(2000))?;
        let found = user_version(&conn)?;
        if found != SCHEMA_VERSION {
            return Err(StoreError::SchemaMismatch {
                found,
                supported: SCHEMA_VERSION,
            });
        }
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
        self.append_batch(&Batch {
            segments: segments.to_vec(),
            ..Batch::default()
        })
    }

    /// Writes a batch in one transaction; an event already stored is skipped.
    pub fn append_batch(&mut self, batch: &Batch) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO segment (series, start_ms, end_ms, data) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for segment in &batch.segments {
                insert.execute(params![
                    segment.series as u8,
                    segment.start_ms,
                    segment.end_ms,
                    segment.data
                ])?;
            }
            let mut insert = tx.prepare_cached(
                "INSERT OR IGNORE INTO event
                 (ts_ms, channel, provider, event_id, record_id, level, meaning, data_xml)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for event in &batch.events {
                insert.execute(params![
                    event.ts_ms,
                    channel_code(event.channel),
                    event.provider,
                    event.id,
                    i64::try_from(event.record_id).unwrap_or(i64::MAX),
                    event.level,
                    meaning_code(event.meaning),
                    event.data_xml
                ])?;
            }
            if let Some(xml) = &batch.bookmark_xml {
                tx.execute(
                    "INSERT INTO event_bookmark (id, xml) VALUES (1, ?1)
                     ON CONFLICT (id) DO UPDATE SET xml = excluded.xml",
                    params![xml],
                )?;
            }
            let mut insert = tx
                .prepare_cached("INSERT INTO gap (start_ms, end_ms, reason) VALUES (?1, ?2, ?3)")?;
            for gap in &batch.gaps {
                insert.execute(params![gap.start_ms, gap.end_ms, gap_code(gap.reason)])?;
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
