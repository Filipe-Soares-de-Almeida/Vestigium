use rusqlite::params;
use vestigium_core::Retention;

use crate::{Series, Store, StoreError};

const HOUR_MS: i64 = 60 * 60_000;
const DAY_MS: i64 = 24 * HOUR_MS;

/// How many rows a prune deleted, per kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub segments: usize,
    pub minutes: usize,
    pub occurrences: usize,
    pub episodes: usize,
    pub events: usize,
    pub gaps: usize,
    pub captures: usize,
}

impl Store {
    /// Deletes what ended before its retention, in one transaction; freed pages are reused.
    pub fn prune(&mut self, now_ms: i64, retention: &Retention) -> Result<PruneReport, StoreError> {
        let samples_cutoff = now_ms - i64::from(retention.samples_hours) * HOUR_MS;
        let minutes_cutoff = now_ms - i64::from(retention.minutes_days) * DAY_MS;
        let records_cutoff = now_ms - i64::from(retention.records_days) * DAY_MS;
        let tx = self.conn.transaction()?;
        let per_second = [
            Series::SamplesV1,
            Series::ProcessesV1,
            Series::Samples,
            Series::Processes,
        ];
        let mut segments = 0;
        for series in per_second {
            segments += tx.execute(
                "DELETE FROM segment WHERE series = ?1 AND end_ms < ?2",
                params![series as u8, samples_cutoff],
            )?;
        }
        let report = PruneReport {
            segments,
            minutes: tx.execute(
                "DELETE FROM segment WHERE series = ?1 AND end_ms < ?2",
                params![Series::Minutes as u8, minutes_cutoff],
            )?,
            occurrences: tx.execute(
                "DELETE FROM occurrence WHERE end_ms < ?1",
                params![records_cutoff],
            )?,
            episodes: tx.execute(
                "DELETE FROM episode WHERE end_ms < ?1",
                params![records_cutoff],
            )?,
            events: tx.execute(
                "DELETE FROM event WHERE ts_ms < ?1",
                params![records_cutoff],
            )?,
            gaps: tx.execute("DELETE FROM gap WHERE end_ms < ?1", params![records_cutoff])?,
            captures: tx.execute(
                "DELETE FROM capture WHERE end_ms < ?1",
                params![records_cutoff],
            )?,
        };
        tx.commit()?;
        Ok(report)
    }

    /// Deletes everything watched and gives the space back; settings, machine facts and the
    /// event bookmark stay, so cleared events are not read again.
    pub fn clear_history(&mut self) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for table in [
            "segment",
            "occurrence",
            "episode",
            "event",
            "gap",
            "capture",
        ] {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
        tx.commit()?;
        // A busy database keeps its size until the next vacuum; the rows are gone either way.
        let _ = self.conn.execute_batch("VACUUM");
        let _ = self
            .conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
        Ok(())
    }

    /// When the last stored sample was taken, to record the gap the agent was stopped for.
    pub fn last_sample_end_ms(&self) -> Result<Option<i64>, StoreError> {
        Ok(self.conn.query_row(
            "SELECT max(end_ms) FROM segment WHERE series IN (?1, ?2)",
            params![Series::SamplesV1 as u8, Series::Samples as u8],
            |row| row.get(0),
        )?)
    }
}
