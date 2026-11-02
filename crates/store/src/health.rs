use rusqlite::{OptionalExtension, Row, params};
use vestigium_core::health::{
    FINDING_VERSION, Finding, FindingChange, SCAN_VERSION, ScanResult, StoredFinding,
};

use crate::{Store, StoreError, decode, encode};

/// A finding with the span it was open: `closed_ms` is `None` while it still holds.
#[derive(Clone, Debug, PartialEq)]
pub struct FindingRecord {
    pub id: i64,
    pub opened_ms: i64,
    pub updated_ms: i64,
    pub closed_ms: Option<i64>,
    pub finding: Finding,
}

impl Store {
    /// Findings that still hold, oldest first.
    pub fn open_findings(&self) -> Result<Vec<StoredFinding>, StoreError> {
        Ok(self
            .finding_rows("WHERE closed_ms IS NULL", params![])?
            .into_iter()
            .map(|r| StoredFinding {
                id: r.id,
                opened_ms: r.opened_ms,
                finding: r.finding,
            })
            .collect())
    }

    /// Findings open at some point of `[from_ms, to_ms]`, oldest first.
    pub fn findings(&self, from_ms: i64, to_ms: i64) -> Result<Vec<FindingRecord>, StoreError> {
        self.finding_rows(
            "WHERE opened_ms <= ?2 AND (closed_ms IS NULL OR closed_ms >= ?1)",
            params![from_ms, to_ms],
        )
    }

    /// Writes one health pass's changes in one transaction.
    pub fn apply_findings(
        &mut self,
        changes: &[FindingChange],
        now_ms: i64,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for change in changes {
            match change {
                FindingChange::Open(finding) => {
                    tx.execute(
                        "INSERT INTO finding (version, opened_ms, updated_ms, closed_ms, data)
                         VALUES (?1, ?2, ?2, NULL, ?3)",
                        params![
                            i64::from(FINDING_VERSION),
                            now_ms,
                            encode(std::slice::from_ref(finding))
                        ],
                    )?;
                }
                FindingChange::Update { id, finding } => {
                    tx.execute(
                        "UPDATE finding SET version = ?2, updated_ms = ?3, data = ?4
                         WHERE id = ?1 AND closed_ms IS NULL",
                        params![
                            id,
                            i64::from(FINDING_VERSION),
                            now_ms,
                            encode(std::slice::from_ref(finding))
                        ],
                    )?;
                }
                FindingChange::Close { id } => {
                    tx.execute(
                        "UPDATE finding SET closed_ms = ?2 WHERE id = ?1 AND closed_ms IS NULL",
                        params![id, now_ms],
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Stores what a full scan read; returns its id.
    pub fn insert_scan(&mut self, scan: &ScanResult) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO scan (version, ts_ms, data) VALUES (?1, ?2, ?3)",
            params![
                i64::from(SCAN_VERSION),
                scan.ts_ms,
                encode(std::slice::from_ref(scan))
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The most recent full scan.
    pub fn last_scan(&self) -> Result<Option<ScanResult>, StoreError> {
        let row: Option<(i64, Vec<u8>)> = self
            .conn
            .query_row(
                "SELECT version, data FROM scan ORDER BY ts_ms DESC, id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(version, data)| one(version, SCAN_VERSION, "scan version", &data))
            .transpose()
    }

    fn finding_rows(
        &self,
        filter: &str,
        args: impl rusqlite::Params,
    ) -> Result<Vec<FindingRecord>, StoreError> {
        let mut query = self.conn.prepare(&format!(
            "SELECT id, opened_ms, updated_ms, closed_ms, version, data FROM finding {filter}
             ORDER BY opened_ms, id"
        ))?;
        let rows = query.query_map(args, |row: &Row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Vec<u8>>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, opened_ms, updated_ms, closed_ms, version, data) = row?;
            Ok(FindingRecord {
                id,
                opened_ms,
                updated_ms,
                closed_ms,
                finding: one(version, FINDING_VERSION, "finding version", &data)?,
            })
        })
        .collect()
    }
}

/// The single item of a BLOB written at `expected` layout.
fn one<T: serde::de::DeserializeOwned>(
    version: i64,
    expected: u8,
    what: &'static str,
    data: &[u8],
) -> Result<T, StoreError> {
    if version != i64::from(expected) {
        return Err(StoreError::UnknownCode {
            what,
            code: version,
        });
    }
    decode::<T>(data)?.pop().ok_or(StoreError::EmptySegment)
}
