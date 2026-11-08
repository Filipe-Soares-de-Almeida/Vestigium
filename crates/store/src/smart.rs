use rusqlite::params;
use vestigium_core::health::DiskHealth;

use crate::{Store, StoreError};

const DAY_MS: i64 = 24 * 60 * 60_000;

/// One day's reading of one SMART attribute of a disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmartReading {
    pub read_ms: i64,
    pub current: u8,
    pub worst: u8,
    pub raw: u64,
}

impl Store {
    /// Keeps a disk's SMART attributes as the reading of its UTC day; a later reading of the same
    /// day replaces it.
    pub fn record_smart(&mut self, health: &DiskHealth) -> Result<(), StoreError> {
        let day = health.read_ms.div_euclid(DAY_MS);
        let tx = self.conn.transaction()?;
        for attr in &health.attributes {
            tx.execute(
                "INSERT OR REPLACE INTO smart_reading
                 (disk, attr, day, read_ms, current, worst, raw)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    health.name,
                    attr.id,
                    day,
                    health.read_ms,
                    attr.current,
                    attr.worst,
                    i64::try_from(attr.raw).unwrap_or(i64::MAX)
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The daily readings of one attribute of a disk taken since `from_ms`, oldest first.
    pub fn smart_history(
        &self,
        disk: &str,
        attr: u8,
        from_ms: i64,
    ) -> Result<Vec<SmartReading>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT read_ms, current, worst, raw FROM smart_reading
             WHERE disk = ?1 AND attr = ?2 AND read_ms >= ?3 ORDER BY day",
        )?;
        let rows = statement.query_map(params![disk, attr, from_ms], |row| {
            Ok(SmartReading {
                read_ms: row.get(0)?,
                current: row.get(1)?,
                worst: row.get(2)?,
                raw: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}
