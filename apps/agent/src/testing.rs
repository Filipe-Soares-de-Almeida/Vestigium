use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::wall_ms;
use vestigium_core::{Channel, EventMeaning, Sample, WinEvent};

/// Polls `done` until it holds or `timeout` passes; CI runners are slow and noisy.
pub(crate) fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    done()
}

pub(crate) fn quiet(ts_ms: i64) -> Sample {
    Sample {
        ts_ms,
        cpu_pct: 20.0,
        dpc_pct: 0.0,
        isr_pct: 0.0,
        perf_limit_pct: Some(100.0),
        proc_perf_pct: Some(100.0),
        mem_total_bytes: 16 << 30,
        mem_avail_bytes: 1 << 30,
        commit_bytes: 20 << 30,
        commit_limit_bytes: 24 << 30,
        hard_faults_per_s: Some(0.0),
        gpu_pct: None,
        probe_max_lag_ms: 1.0,
        sampler_lag_ms: 0.0,
        cores: Vec::new(),
        disks: Vec::new(),
    }
}

pub(crate) fn disk_event(record_id: u64) -> WinEvent {
    WinEvent {
        ts_ms: wall_ms(),
        channel: Channel::System,
        provider: "disk".to_string(),
        id: 153,
        record_id,
        level: 3,
        meaning: EventMeaning::DiskRetry,
        data_xml: String::new(),
    }
}

/// Holds the writer lock of the history file in `dir` until it is dropped.
pub(crate) struct WriterLock(rusqlite::Connection);

impl WriterLock {
    pub(crate) fn take(dir: &std::path::Path) -> WriterLock {
        let conn = rusqlite::Connection::open(dir.join("vestigium.db")).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        WriterLock(conn)
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = self.0.execute_batch("ROLLBACK");
    }
}

/// Makes every insert into `table` fail until the returned guard is dropped.
pub(crate) struct Refusal(rusqlite::Connection, &'static str);

impl Refusal {
    pub(crate) fn of(dir: &std::path::Path, table: &'static str) -> Refusal {
        let conn = rusqlite::Connection::open(dir.join("vestigium.db")).unwrap();
        conn.execute_batch(&format!(
            "CREATE TRIGGER refuse_{table} BEFORE INSERT ON {table}
             BEGIN SELECT RAISE(ABORT, 'refused'); END"
        ))
        .unwrap();
        Refusal(conn, table)
    }
}

impl Drop for Refusal {
    fn drop(&mut self) {
        let _ = self
            .0
            .execute_batch(&format!("DROP TRIGGER refuse_{}", self.1));
    }
}
