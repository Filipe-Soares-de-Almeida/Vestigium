use std::collections::BTreeMap;

use vestigium_collect::RingContents;
use vestigium_core::{
    CrashKind, EventMeaning, MachineInfo, Occurrence, POSTMORTEM_BEFORE_MS, POSTMORTEM_REACH_MS,
    WinEvent, Window, postmortem, power_loss_detail, unexpected_shutdown_ms,
};
use vestigium_store::{Store, StoreError};

/// How long a start waits for Windows to log a fall before calling it a plain stop.
pub const WAIT_FOR_FALL_MS: i64 = 5 * 60_000;
/// After the Kernel-Power 41, how long to wait for the EventLog 6008 that dates the fall.
const WAIT_FOR_DATE_MS: i64 = 30_000;
/// How long after the last second watched the Kernel-Power 41 may be stamped when no EventLog 6008
/// dates the fall: the post-mortem reach plus the time a boot takes to log it.
const UNDATED_REACH_MS: i64 = POSTMORTEM_REACH_MS + 2 * 60_000;
/// What the next boot logs about a fall, kept with it.
const NEXT_BOOT: [EventMeaning; 5] = [
    EventMeaning::PowerLoss,
    EventMeaning::UnexpectedShutdown,
    EventMeaning::BugCheck,
    EventMeaning::HardwareError,
    EventMeaning::NoCrashDump,
];

const CALM_SYNC_MS: i64 = 60_000;
const TENSE_SYNC_MS: i64 = 5_000;
/// A stall keeps the sync short for this long after it.
const TENSE_FOR_MS: i64 = 60_000;
const TENSE_MEMORY_PCT: f32 = 90.0;
/// A stall a person notices.
const STALL_MS: f32 = 50.0;

/// A start that found the black box not closed cleanly, waiting for Windows to say whether the PC fell.
#[derive(Debug)]
pub struct Fall {
    ring: RingContents,
    /// The last second watched, from the ring or the history, whichever is newer.
    last_ms: i64,
    deadline_ms: i64,
    /// What the Kernel-Power 41 said, when it arrived and the time it is stamped with.
    loss: Option<Loss>,
    down_ms: Option<i64>,
    next_boot: Vec<WinEvent>,
    /// Events logged shortly before the last second watched, delivered again by the old bookmark.
    before: Vec<WinEvent>,
}

/// The Kernel-Power 41 of a fall.
#[derive(Debug)]
struct Loss {
    seen_ms: i64,
    stamp_ms: i64,
    kind: CrashKind,
}

impl Fall {
    /// `None` when the ring was closed cleanly or nothing was ever watched.
    pub fn after(ring: RingContents, history_last_ms: Option<i64>, now_ms: i64) -> Option<Fall> {
        if ring.clean {
            return None;
        }
        let ring_last = ring.samples.last().map(|s| s.ts_ms);
        let last_ms = ring_last.max(history_last_ms)?;
        Some(Fall {
            ring,
            last_ms,
            deadline_ms: now_ms + WAIT_FOR_FALL_MS,
            loss: None,
            down_ms: None,
            next_boot: Vec::new(),
            before: Vec::new(),
        })
    }

    /// The last second watched before the stop.
    pub fn last_ms(&self) -> i64 {
        self.last_ms
    }

    /// Takes in a delivered event: what the next boot logged after the last second watched dates
    /// and classifies the fall, and what was logged just before it joins the evidence window.
    pub fn offer(&mut self, event: &WinEvent, now_ms: i64) {
        if event.ts_ms <= self.last_ms {
            if event.ts_ms >= self.last_ms - POSTMORTEM_BEFORE_MS
                && !self.before.iter().any(|kept| same_record(kept, event))
            {
                self.before.push(event.clone());
            }
            return;
        }
        if !NEXT_BOOT.contains(&event.meaning) {
            return;
        }
        match event.meaning {
            EventMeaning::PowerLoss if self.loss.is_none() => {
                self.loss = power_loss_detail(&event.data_xml).map(|d| Loss {
                    seen_ms: now_ms,
                    stamp_ms: event.ts_ms,
                    kind: d.crash_kind(),
                });
            }
            EventMeaning::UnexpectedShutdown if self.down_ms.is_none() => {
                self.down_ms = unexpected_shutdown_ms(&event.data_xml);
            }
            _ => {}
        }
        self.next_boot.push(event.clone());
    }

    /// Waiting is over: the fall is dated, its date did not come, or Windows logged no fall at all.
    pub fn settled(&self, now_ms: i64) -> bool {
        now_ms >= self.deadline_ms
            || self.loss.as_ref().is_some_and(|loss| {
                self.down_ms.is_some() || now_ms - loss.seen_ms >= WAIT_FOR_DATE_MS
            })
    }

    /// The post-mortem, from the history and the ring; `None` when the PC did not fall while watched.
    pub fn occurrence(self, store: &Store) -> Result<Option<Occurrence>, StoreError> {
        let Some(Loss { stamp_ms, kind, .. }) = self.loss else {
            return Ok(None);
        };
        // Without a 6008 only the 41's own stamp can show the fall came right after the last second.
        if self.down_ms.is_none() && stamp_ms - self.last_ms > UNDATED_REACH_MS {
            return Ok(None);
        }
        let from = self.last_ms - POSTMORTEM_BEFORE_MS;
        let to = self.last_ms;
        // The history wins where both hold a second: it keeps every core and disk.
        let mut samples: BTreeMap<i64, _> = BTreeMap::new();
        let in_window = |ts: i64| (from..=to).contains(&ts);
        for s in self.ring.samples.into_iter().filter(|s| in_window(s.ts_ms)) {
            samples.insert(s.ts_ms, s);
        }
        for s in store.samples(from, to)? {
            samples.insert(s.ts_ms, s);
        }
        let mut tops: BTreeMap<i64, _> = BTreeMap::new();
        for t in self.ring.tops.into_iter().filter(|t| in_window(t.ts_ms)) {
            tops.insert(t.ts_ms, t);
        }
        for t in store.tops(from, to)? {
            tops.insert(t.ts_ms, t);
        }
        let machine = store.machine()?.unwrap_or(MachineInfo {
            logical_cpus: 0,
            mem_total_bytes: 0,
            disks: Vec::new(),
        });
        let mut events = store.events(from, to)?;
        for kept in self.before {
            if !events.iter().any(|stored| same_record(stored, &kept)) {
                events.push(kept);
            }
        }
        events.sort_by_key(|e| e.ts_ms);
        let window = Window {
            from_ms: from,
            to_ms: to,
            samples: samples.into_values().collect(),
            tops: tops.into_values().collect(),
            events,
            hangs: Vec::new(),
            gaps: store.gaps(from, to)?,
            machine,
        };
        let down_ms = self.down_ms.unwrap_or(self.last_ms);
        Ok(postmortem(
            window,
            self.last_ms,
            down_ms,
            kind,
            self.next_boot,
        ))
    }
}

/// An event is unique per channel and record id.
fn same_record(a: &WinEvent, b: &WinEvent) -> bool {
    a.channel == b.channel && a.record_id == b.record_id
}

/// What makes the black box sync sooner, read once a second.
#[derive(Clone, Copy, Debug, Default)]
pub struct Strain {
    pub pressure_open: bool,
    pub mem_used_pct: f32,
    pub worst_lag_ms: f32,
    /// Windows logged a hardware error since the agent started.
    pub hardware_error: bool,
}

/// When the black box goes to disk: every minute while calm, every 5 s under strain.
#[derive(Debug)]
pub struct RingSync {
    synced_ms: i64,
    tense_until_ms: i64,
}

impl RingSync {
    pub fn new(now_ms: i64) -> RingSync {
        RingSync {
            synced_ms: now_ms,
            tense_until_ms: i64::MIN,
        }
    }

    /// True when a sync is due now; it then counts as done.
    pub fn due(&mut self, now_ms: i64, strain: Strain) -> bool {
        if strain.worst_lag_ms >= STALL_MS {
            self.tense_until_ms = now_ms + TENSE_FOR_MS;
        }
        let tense = strain.pressure_open
            || strain.hardware_error
            || strain.mem_used_pct >= TENSE_MEMORY_PCT
            || now_ms < self.tense_until_ms;
        let every = if tense { TENSE_SYNC_MS } else { CALM_SYNC_MS };
        if now_ms - self.synced_ms < every {
            return false;
        }
        self.synced_ms = now_ms;
        true
    }
}

#[cfg(test)]
mod tests {
    use vestigium_core::Channel;
    use vestigium_store::Batch;

    use super::*;

    fn event(ts_ms: i64, meaning: EventMeaning, data_xml: &str) -> WinEvent {
        WinEvent {
            ts_ms,
            channel: Channel::System,
            provider: "Microsoft-Windows-Kernel-Power".into(),
            id: 41,
            record_id: ts_ms as u64,
            level: 1,
            meaning,
            data_xml: data_xml.into(),
        }
    }

    fn unclean() -> RingContents {
        RingContents::default()
    }

    const BLUE_SCREEN: &str = "<EventData><Data Name='BugcheckCode'>80</Data>\
        <Data Name='SleepInProgress'>0</Data><Data Name='PowerButtonTimestamp'>0</Data></EventData>";

    #[test]
    fn tension_shortens_the_sync() {
        let calm = Strain::default();
        let mut sync = RingSync::new(0);
        assert!(!sync.due(5_000, calm));
        assert!(!sync.due(59_000, calm));
        assert!(sync.due(60_000, calm));

        let full = Strain {
            mem_used_pct: 93.0,
            ..calm
        };
        assert!(sync.due(65_000, full));
        assert!(!sync.due(67_000, full));
        assert!(sync.due(70_000, full));

        // A stall keeps the short sync for a minute after it, then the calm one comes back.
        let stall = Strain {
            worst_lag_ms: 400.0,
            ..calm
        };
        assert!(sync.due(75_000, stall));
        assert!(sync.due(80_000, calm));
        assert!(!sync.due(139_000, calm));
        assert!(sync.due(140_000, calm));
    }

    #[test]
    fn a_clean_ring_is_no_fall() {
        let clean = RingContents {
            clean: true,
            ..RingContents::default()
        };
        assert!(Fall::after(clean, Some(1_000), 2_000).is_none());
        assert!(Fall::after(unclean(), None, 2_000).is_none());
    }

    #[test]
    fn a_fall_waits_for_its_date_then_settles() {
        let mut fall = Fall::after(unclean(), Some(100_000), 200_000).unwrap();
        // A 41 from before the last watched second is history, not this fall.
        fall.offer(
            &event(50_000, EventMeaning::PowerLoss, BLUE_SCREEN),
            200_000,
        );
        assert!(!fall.settled(200_000));
        fall.offer(
            &event(400_000, EventMeaning::PowerLoss, BLUE_SCREEN),
            201_000,
        );
        assert!(!fall.settled(201_000));
        assert!(
            fall.settled(231_000),
            "the 6008 never came; the 41 alone settles it"
        );
    }

    fn disk_event(ts_ms: i64, record_id: u64) -> WinEvent {
        WinEvent {
            record_id,
            provider: "disk".into(),
            id: 153,
            ..event(ts_ms, EventMeaning::DiskRetry, "")
        }
    }

    fn settled_occurrence(mut fall: Fall, store: &Store) -> Occurrence {
        let after = fall.last_ms() + 1_000;
        fall.offer(&event(after, EventMeaning::PowerLoss, BLUE_SCREEN), 1);
        fall.occurrence(store).unwrap().unwrap()
    }

    #[test]
    fn events_before_the_last_second_are_kept_for_the_window() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let last = 1_000_000;
        let mut fall = Fall::after(unclean(), Some(last), 2_000_000).unwrap();
        fall.offer(&disk_event(last - 90_000, 1), 2_000_000);
        fall.offer(&disk_event(last, 2), 2_000_000);

        let occurrence = settled_occurrence(fall, &store);
        let kept: Vec<i64> = occurrence
            .window
            .events
            .iter()
            .filter(|e| e.meaning == EventMeaning::DiskRetry)
            .map(|e| e.ts_ms)
            .collect();
        assert_eq!(kept, [last - 90_000, last]);
    }

    #[test]
    fn events_older_than_the_window_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let last = 1_000_000;
        let mut fall = Fall::after(unclean(), Some(last), 2_000_000).unwrap();
        fall.offer(&disk_event(last - POSTMORTEM_BEFORE_MS - 1, 1), 2_000_000);

        let occurrence = settled_occurrence(fall, &store);
        assert!(
            occurrence
                .window
                .events
                .iter()
                .all(|e| e.meaning != EventMeaning::DiskRetry)
        );
    }

    #[test]
    fn a_stored_event_and_a_kept_one_with_the_same_record_are_counted_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let last = 1_000_000;
        let stored = disk_event(last - 30_000, 7);
        store
            .append_batch(&Batch {
                events: vec![stored.clone()],
                ..Batch::default()
            })
            .unwrap();
        let mut fall = Fall::after(unclean(), Some(last), 2_000_000).unwrap();
        fall.offer(&stored, 2_000_000);
        fall.offer(&stored, 2_000_000);

        let occurrence = settled_occurrence(fall, &store);
        let disk = occurrence
            .window
            .events
            .iter()
            .filter(|e| e.meaning == EventMeaning::DiskRetry)
            .count();
        assert_eq!(disk, 1);
    }

    #[test]
    fn no_word_from_windows_settles_as_a_plain_stop() {
        let fall = Fall::after(unclean(), Some(100_000), 200_000).unwrap();
        assert!(!fall.settled(200_000 + WAIT_FOR_FALL_MS - 1));
        assert!(fall.settled(200_000 + WAIT_FOR_FALL_MS));
    }

    fn dateless_occurrence(stamp_after_last_ms: i64) -> Option<Occurrence> {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let last = 1_000_000;
        let mut fall = Fall::after(unclean(), Some(last), 2_000_000).unwrap();
        fall.offer(
            &event(
                last + stamp_after_last_ms,
                EventMeaning::PowerLoss,
                BLUE_SCREEN,
            ),
            2_000_000,
        );
        assert!(fall.settled(2_000_000 + WAIT_FOR_DATE_MS));
        fall.occurrence(&store).unwrap()
    }

    #[test]
    fn a_fall_without_a_date_is_a_stop_when_the_41_comes_long_after_the_last_second() {
        assert!(dateless_occurrence(45 * 60_000).is_none());
        assert!(dateless_occurrence(UNDATED_REACH_MS + 1).is_none());
    }

    #[test]
    fn a_fall_without_a_date_is_kept_when_the_41_comes_soon() {
        assert!(dateless_occurrence(2 * 60_000).is_some());
        assert!(dateless_occurrence(UNDATED_REACH_MS).is_some());
    }

    #[test]
    fn a_fall_with_a_date_ignores_how_late_the_41_came() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("vestigium.db")).unwrap();
        let last = 1_000_000;
        let mut fall = Fall::after(unclean(), Some(last), 2_000_000).unwrap();
        fall.offer(
            &event(last + 45 * 60_000, EventMeaning::PowerLoss, BLUE_SCREEN),
            2_000_000,
        );
        fall.down_ms = Some(last + 1_000);
        assert!(fall.occurrence(&store).unwrap().is_some());
    }
}
