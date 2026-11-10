use rusqlite::{OptionalExtension, params};
use vestigium_core::health::Subject;
use vestigium_core::hwtest::{Outcome, TEST_RUN_VERSION, TestKind, TestRun, TestRunV1, TestRunV2};

use crate::{Store, StoreError, decode, encode};

/// One run of a hardware test as the history keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct TestRunRow {
    pub id: i64,
    pub run: TestRun,
}

fn kind_code(kind: TestKind) -> i64 {
    match kind {
        TestKind::MemoryQuick => 0,
        TestKind::DiskSurface => 1,
        TestKind::VolumeCheck => 2,
        TestKind::DiskBenchmark => 3,
    }
}

fn outcome_code(outcome: Outcome) -> i64 {
    match outcome {
        Outcome::Clean => 0,
        Outcome::Problems => 1,
        Outcome::Interrupted => 2,
        Outcome::Failed => 3,
    }
}

/// The subject as a key the table can match on.
fn subject_key(subject: &Subject) -> String {
    match subject {
        Subject::Pc => "pc".into(),
        Subject::Disk(name) => format!("disk:{name}"),
        Subject::Volume(root) => format!("volume:{root}"),
    }
}

impl Store {
    /// Keeps one run; the same run imported again keeps its first row. Returns the row's id.
    pub fn record_test_run(&mut self, run: &TestRun) -> Result<i64, StoreError> {
        let kind = kind_code(run.kind());
        let subject = subject_key(&run.subject);
        let tx = self.conn.transaction()?;
        let known: Option<i64> = tx
            .query_row(
                "SELECT id FROM test_run WHERE kind = ?1 AND subject = ?2 AND started_ms = ?3",
                params![kind, subject, run.started_ms],
                |row| row.get(0),
            )
            .optional()?;
        let id = match known {
            Some(id) => id,
            None => {
                tx.execute(
                    "INSERT INTO test_run (version, kind, subject, started_ms, ended_ms, outcome,
                         data)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        i64::from(TEST_RUN_VERSION),
                        kind,
                        subject,
                        run.started_ms,
                        run.ended_ms,
                        outcome_code(run.outcome),
                        encode(std::slice::from_ref(run))
                    ],
                )?;
                tx.last_insert_rowid()
            }
        };
        tx.commit()?;
        Ok(id)
    }

    /// Runs that ended in `[from_ms, to_ms]`, oldest first; a row of an unknown layout is skipped and
    /// one of version 2 or 1 is read as a run with the fields it lacks empty.
    pub fn test_runs(&self, from_ms: i64, to_ms: i64) -> Result<Vec<TestRunRow>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, version, data FROM test_run WHERE ended_ms BETWEEN ?1 AND ?2
             ORDER BY started_ms, id",
        )?;
        let rows = statement.query_map(params![from_ms, to_ms], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })?;
        let mut runs = Vec::new();
        for row in rows {
            let (id, version, data) = row?;
            let run = if version == i64::from(TEST_RUN_VERSION) {
                decode::<TestRun>(&data)?.pop()
            } else if version == 2 {
                decode::<TestRunV2>(&data)?.pop().map(TestRun::from)
            } else if version == 1 {
                decode::<TestRunV1>(&data)?.pop().map(TestRun::from)
            } else {
                continue;
            };
            if let Some(run) = run {
                runs.push(TestRunRow { id, run });
            }
        }
        Ok(runs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_and_outcome_codes_are_fixed() {
        let kinds: Vec<i64> = TestKind::ALL.into_iter().map(kind_code).collect();
        let outcomes: Vec<i64> = Outcome::ALL.into_iter().map(outcome_code).collect();
        assert_eq!(kinds, [0, 1, 2, 3]);
        assert_eq!(outcomes, [0, 1, 2, 3]);
    }
}
