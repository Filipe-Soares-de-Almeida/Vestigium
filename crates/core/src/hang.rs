use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{HangOutcome, Trigger};

/// How long a window goes without reading input before Windows calls it not responding.
pub const HUNG_AFTER_MS: i64 = 5_000;

/// A process with a window Windows reports as not responding; window titles are never kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HungWindow {
    pub pid: u32,
    pub name: String,
}

/// Turns the periodic not-responding checks into one trigger per hang.
#[derive(Debug, Default)]
pub struct HangTracker {
    /// Hung processes by pid, with their name and the first check that saw them.
    open: HashMap<u32, (String, i64)>,
}

impl HangTracker {
    pub fn new() -> HangTracker {
        HangTracker::default()
    }

    /// The hangs that ended by this check: recovered when the process still runs, closed otherwise.
    pub fn push(
        &mut self,
        ts_ms: i64,
        hung: &[HungWindow],
        running: &HashSet<u32>,
    ) -> Vec<Trigger> {
        let now_hung: HashSet<u32> = hung.iter().map(|h| h.pid).collect();
        let mut ended: Vec<(i64, u32)> = self
            .open
            .iter()
            .filter(|(pid, _)| !now_hung.contains(pid))
            .map(|(&pid, &(_, first_seen))| (first_seen, pid))
            .collect();
        ended.sort_unstable();
        let triggers: Vec<Trigger> = ended
            .into_iter()
            .map(|(_, pid)| pid)
            .filter_map(|pid| {
                let (name, first_seen) = self.open.remove(&pid)?;
                Some(Trigger::AppHang {
                    pid,
                    name,
                    start_ms: first_seen - HUNG_AFTER_MS,
                    end_ms: ts_ms,
                    outcome: if running.contains(&pid) {
                        HangOutcome::Recovered
                    } else {
                        HangOutcome::Closed
                    },
                })
            })
            .collect();
        for window in hung {
            self.open
                .entry(window.pid)
                .or_insert_with(|| (window.name.clone(), ts_ms));
        }
        triggers
    }

    /// Forgets every open hang, as when watching pauses.
    pub fn reset(&mut self) {
        self.open.clear();
    }
}
