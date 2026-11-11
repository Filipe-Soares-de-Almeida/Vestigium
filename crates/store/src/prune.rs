use rusqlite::params;
use vestigium_core::Retention;

use crate::segment::MAX_SEGMENT_SPAN_MS;
use crate::{Series, Store, StoreError};

const HOUR_MS: i64 = 60 * 60_000;
const DAY_MS: i64 = 24 * HOUR_MS;

/// How far past the newest stored data a prune may run: the interval between prunes.
const MAX_AHEAD_MS: i64 = HOUR_MS;

/// Start of the newest stored segment.
const NEWEST_START_SQL: &str = "SELECT max(start_ms) FROM segment";
/// End of the newest stored segment, given the start of the newest one in `?1`.
const NEWEST_END_SQL: &str = "SELECT max(end_ms) FROM segment WHERE start_ms >= ?1";
/// Deletes the segments of one series that ended before a cutoff; a segment that ended before it
/// also began before it, which is what lets the index find them.
const PRUNE_SEGMENTS_SQL: &str =
    "DELETE FROM segment WHERE series = ?1 AND start_ms < ?2 AND end_ms < ?2";

/// How many rows a prune deleted, per kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PruneReport {
    pub refused: bool,
    pub segments: usize,
    pub minutes: usize,
    pub occurrences: usize,
    pub episodes: usize,
    pub events: usize,
    pub gaps: usize,
    pub captures: usize,
    pub findings: usize,
    pub scans: usize,
    pub dumps: usize,
    pub tweak_changes: usize,
    pub test_runs: usize,
    pub smart_readings: usize,
}

impl Store {
    /// Deletes what ended before its retention, in one transaction; freed pages are reused.
    /// Deletes nothing and reports `refused` when `now_ms` is over an hour past the newest segment.
    pub fn prune(&mut self, now_ms: i64, retention: &Retention) -> Result<PruneReport, StoreError> {
        let samples_cutoff = now_ms - i64::from(retention.samples_hours) * HOUR_MS;
        let minutes_cutoff = now_ms - i64::from(retention.minutes_days) * DAY_MS;
        let records_cutoff = now_ms - i64::from(retention.records_days) * DAY_MS;
        let tx = self.conn.transaction()?;
        let newest_start: Option<i64> = tx.query_row(NEWEST_START_SQL, [], |row| row.get(0))?;
        let newest: Option<i64> = match newest_start {
            Some(start_ms) => tx.query_row(
                NEWEST_END_SQL,
                [start_ms.saturating_sub(MAX_SEGMENT_SPAN_MS)],
                |row| row.get(0),
            )?,
            None => None,
        };
        if newest.is_some_and(|end_ms| now_ms - end_ms > MAX_AHEAD_MS) {
            return Ok(PruneReport {
                refused: true,
                ..PruneReport::default()
            });
        }
        let per_second = [
            Series::SamplesV1,
            Series::ProcessesV1,
            Series::SamplesV2,
            Series::Samples,
            Series::Processes,
        ];
        let mut segments = 0;
        for series in per_second {
            segments += tx.execute(PRUNE_SEGMENTS_SQL, params![series as u8, samples_cutoff])?;
        }
        let report = PruneReport {
            refused: false,
            segments,
            minutes: tx.execute(
                PRUNE_SEGMENTS_SQL,
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
            findings: tx.execute(
                "DELETE FROM finding WHERE closed_ms < ?1",
                params![records_cutoff],
            )?,
            scans: tx.execute(
                "DELETE FROM scan WHERE ts_ms < ?1
                 AND id <> (SELECT id FROM scan ORDER BY ts_ms DESC, id DESC LIMIT 1)",
                params![records_cutoff],
            )?,
            dumps: tx.execute(
                "DELETE FROM dump WHERE deleted_ms < ?1",
                params![records_cutoff],
            )?,
            // A change still in place stays at any age, so it can always be undone.
            tweak_changes: tx.execute(
                "DELETE FROM tweak_change WHERE applied_ms < ?1
                 AND (undone_ms IS NOT NULL OR undo_of IS NOT NULL)",
                params![records_cutoff],
            )?,
            test_runs: tx.execute(
                "DELETE FROM test_run WHERE ended_ms < ?1",
                params![records_cutoff],
            )?,
            smart_readings: tx.execute(
                "DELETE FROM smart_reading WHERE read_ms < ?1",
                params![records_cutoff],
            )?,
        };
        tx.commit()?;
        Ok(report)
    }

    /// Deletes everything watched and gives the space back; settings, machine facts, the event
    /// bookmark and the tweak history stay, so events are not read again and changes can be undone.
    pub fn clear_history(&mut self) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        for table in [
            "segment",
            "occurrence",
            "episode",
            "event",
            "gap",
            "capture",
            "finding",
            "scan",
            "dump",
            "test_run",
            "smart_reading",
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
            "SELECT max(end_ms) FROM segment WHERE series IN (?1, ?2, ?3)",
            params![
                Series::SamplesV1 as u8,
                Series::SamplesV2 as u8,
                Series::Samples as u8
            ],
            |row| row.get(0),
        )?)
    }

    /// When the oldest history still stored begins, of any resolution.
    pub fn first_segment_ms(&self) -> Result<Option<i64>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT min(start_ms) FROM segment", [], |row| row.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SEGMENTS_SQL;

    /// The plan lines SQLite reports for a statement.
    fn plan(store: &Store, sql: &str, params: impl rusqlite::Params) -> Vec<String> {
        let mut query = store
            .conn
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        query
            .query_map(params, |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn assert_no_scan(name: &str, plan: &[String]) {
        assert!(
            !plan.iter().any(|line| line.starts_with("SCAN")),
            "{name} scans the table: {plan:?}"
        );
    }

    #[test]
    fn reading_and_pruning_segments_search_an_index() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("plans.db")).unwrap();

        let read = plan(&store, SEGMENTS_SQL, params![3, 0, 1_000_000, 0]);
        assert_no_scan("segments", &read);
        assert!(
            read.iter()
                .any(|line| line.contains("segment_series_start")),
            "segments does not use the series index: {read:?}"
        );

        let prune = plan(&store, PRUNE_SEGMENTS_SQL, params![3, 1_000_000]);
        assert_no_scan("segment prune", &prune);
        assert!(
            prune
                .iter()
                .any(|line| line.contains("segment_series_start")),
            "the segment prune does not use the series index: {prune:?}"
        );

        assert_no_scan("newest start", &plan(&store, NEWEST_START_SQL, []));
        assert_no_scan("newest end", &plan(&store, NEWEST_END_SQL, [0]));
    }
}
