use rusqlite::{OptionalExtension, params};
use vestigium_core::health::{BOOT_VERSION, BootFacts, TempSize};

use crate::{Store, StoreError, decode, encode};

impl Store {
    /// Stores a boot, or rewrites the stored one with the same start: a boot read again later
    /// carries the events logged after the first read, such as the startup apps.
    pub fn put_boot(&mut self, boot: &BootFacts) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO boot (version, start_ms, data) VALUES (?1, ?2, ?3)
             ON CONFLICT (start_ms) DO UPDATE SET version = excluded.version, data = excluded.data",
            params![
                i64::from(BOOT_VERSION),
                boot.kernel_start_ms,
                encode(std::slice::from_ref(boot))
            ],
        )?;
        Ok(())
    }

    /// The latest `limit` boots, oldest first.
    pub fn boots(&self, limit: u32) -> Result<Vec<BootFacts>, StoreError> {
        let mut query = self
            .conn
            .prepare("SELECT version, data FROM boot ORDER BY start_ms DESC LIMIT ?1")?;
        let rows = query.query_map(params![i64::from(limit)], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;
        let mut boots = rows
            .map(|row| {
                let (version, data) = row?;
                if version != i64::from(BOOT_VERSION) {
                    return Err(StoreError::UnknownCode {
                        what: "boot version",
                        code: version,
                    });
                }
                decode::<BootFacts>(&data)?
                    .pop()
                    .ok_or(StoreError::EmptySegment)
            })
            .collect::<Result<Vec<_>, _>>()?;
        boots.reverse();
        Ok(boots)
    }

    /// When the newest stored boot started, to tell whether a boot is new.
    pub fn last_boot_start(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT max(start_ms) FROM boot", [], |row| row.get(0))?)
    }

    /// Stores one count of the Temp folder.
    pub fn put_temp(&mut self, size: &TempSize) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO temp_reading (read_ms, folders, files, capped)
             VALUES (?1, ?2, ?3, ?4)",
            params![size.ts_ms, size.folders, size.files, size.capped],
        )?;
        Ok(())
    }

    /// The newest count of the Temp folder.
    pub fn last_temp(&self) -> Result<Option<TempSize>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT read_ms, folders, files, capped FROM temp_reading
                 ORDER BY read_ms DESC LIMIT 1",
                [],
                |row| {
                    Ok(TempSize {
                        ts_ms: row.get(0)?,
                        folders: row.get(1)?,
                        files: row.get(2)?,
                        capped: row.get(3)?,
                    })
                },
            )
            .optional()?)
    }
}
