use std::mem;
use std::time::Duration;

use vestigium_collect::{
    CollectError, CpuReading, CpuSampler, HighResTimer, MemoryInfo, PdhReading, PdhSampler,
    ProcessSnapshotter, Waited, Wake, memory_info,
};
use vestigium_core::ProcessCounters;
use vestigium_ipc::{MonitorSource, SourceFailure};

const SNAPSHOT_BUFFER: usize = 512 * 1024;

/// Blocks the sampler thread until the next tick.
pub trait Waiter {
    /// Waits `due`, or until `wake` is set, and says which came first.
    fn wait(&self, due: Duration, wake: &Wake) -> Result<Waited, CollectError>;
}

impl Waiter for HighResTimer {
    fn wait(&self, due: Duration, wake: &Wake) -> Result<Waited, CollectError> {
        self.wait_or_wake(due, wake)
    }
}

/// What one sampler tick reads and how the performance counters and the timer are opened.
pub trait Readings {
    fn memory(&mut self) -> Result<MemoryInfo, CollectError>;
    /// `None` on the first reading after a start or a reset, which has no rates.
    fn cpu(&mut self) -> Result<Option<CpuReading>, CollectError>;
    /// `None` on the first reading after an open or a reset, which has no rates.
    fn pdh(&mut self) -> Result<Option<PdhReading>, CollectError>;
    fn processes(&mut self) -> Result<Vec<ProcessCounters>, CollectError>;
    /// Makes the next CPU and counter readings baselines again, as after a pause.
    fn reset(&mut self);
    fn open_pdh(&mut self) -> Result<(), CollectError>;
    fn open_timer(&mut self) -> Result<Box<dyn Waiter>, CollectError>;
}

/// Builds the sources on the sampler thread, where their handles stay.
pub type SourceSet = Box<dyn FnOnce() -> Box<dyn Readings> + Send>;

/// The set that reads this PC.
pub fn windows_source_set() -> SourceSet {
    Box::new(|| Box::new(WindowsReadings::new()))
}

/// The Windows sources: the memory call, the processor times, the PDH query and the process list.
pub struct WindowsReadings {
    cpu: CpuSampler,
    pdh: Option<PdhSampler>,
    processes: ProcessSnapshotter,
}

impl WindowsReadings {
    pub fn new() -> WindowsReadings {
        WindowsReadings {
            cpu: CpuSampler::new(),
            pdh: None,
            processes: ProcessSnapshotter::with_capacity(SNAPSHOT_BUFFER),
        }
    }
}

impl Readings for WindowsReadings {
    fn memory(&mut self) -> Result<MemoryInfo, CollectError> {
        memory_info()
    }

    fn cpu(&mut self) -> Result<Option<CpuReading>, CollectError> {
        self.cpu.sample()
    }

    fn pdh(&mut self) -> Result<Option<PdhReading>, CollectError> {
        self.pdh.as_mut().map_or(Ok(None), PdhSampler::sample)
    }

    fn processes(&mut self) -> Result<Vec<ProcessCounters>, CollectError> {
        self.processes.snapshot()
    }

    fn reset(&mut self) {
        self.cpu.reset();
        if let Some(pdh) = self.pdh.as_mut() {
            pdh.reset();
        }
    }

    fn open_pdh(&mut self) -> Result<(), CollectError> {
        self.pdh = Some(PdhSampler::open()?);
        Ok(())
    }

    fn open_timer(&mut self) -> Result<Box<dyn Waiter>, CollectError> {
        Ok(Box::new(HighResTimer::new()?))
    }
}

/// Longest wait, in ticks, between two attempts to open a source again.
const REOPEN_CAP: u32 = 60;

/// Spaces the attempts to open a source again, in ticks: 1, 2, 4 and so on up to a cap.
#[derive(Debug)]
pub struct Backoff {
    wait: u32,
    left: u32,
}

impl Backoff {
    pub fn new() -> Backoff {
        Backoff { wait: 1, left: 0 }
    }

    /// True when an attempt is due this tick.
    pub fn due(&mut self) -> bool {
        if self.left == 0 {
            return true;
        }
        self.left -= 1;
        false
    }

    /// Notes a failed attempt: the next one comes after a longer wait.
    pub fn failed(&mut self) {
        self.left = self.wait - 1;
        self.wait = (self.wait * 2).min(REOPEN_CAP);
    }
}

/// The sources whose health is tracked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Memory,
    Cpu,
    Pdh,
    Processes,
    Timer,
}

const SOURCES: usize = 5;

/// Whether a source reads, or since when and why it does not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum SourceHealth {
    #[default]
    Working,
    Failing {
        since_ms: i64,
        last_error: String,
        failures: u32,
    },
}

impl Source {
    const ALL: [Source; SOURCES] = [
        Source::Memory,
        Source::Cpu,
        Source::Pdh,
        Source::Processes,
        Source::Timer,
    ];

    fn monitored(self) -> MonitorSource {
        match self {
            Source::Memory => MonitorSource::Memory,
            Source::Cpu => MonitorSource::Cpu,
            Source::Pdh => MonitorSource::Counters,
            Source::Processes => MonitorSource::Processes,
            Source::Timer => MonitorSource::Timer,
        }
    }
}

/// The health of every source, updated where each one is read.
#[derive(Debug, Default)]
pub struct SourceHealths {
    sources: [SourceHealth; SOURCES],
    /// Set when a source started or stopped failing, or changed its error, until taken.
    changed: bool,
}

impl SourceHealths {
    /// Notes how a read went; returns the error text when it is new for this source.
    pub fn record<T>(
        &mut self,
        source: Source,
        now_ms: i64,
        result: &Result<T, CollectError>,
    ) -> Option<String> {
        let health = &mut self.sources[source as usize];
        let Err(err) = result else {
            self.changed |= *health != SourceHealth::Working;
            *health = SourceHealth::Working;
            return None;
        };
        let text = err.to_string();
        match health {
            SourceHealth::Failing {
                last_error,
                failures,
                ..
            } => {
                *failures += 1;
                if *last_error == text {
                    return None;
                }
                last_error.clone_from(&text);
                self.changed = true;
            }
            SourceHealth::Working => {
                self.changed = true;
                *health = SourceHealth::Failing {
                    since_ms: now_ms,
                    last_error: text.clone(),
                    failures: 1,
                };
            }
        }
        Some(text)
    }

    /// True once after a source started failing, recovered or changed its error.
    pub fn take_changed(&mut self) -> bool {
        mem::take(&mut self.changed)
    }

    /// The sources failing now, with since when and why.
    pub fn failing(&self) -> Vec<SourceFailure> {
        Source::ALL
            .into_iter()
            .filter_map(|source| match &self.sources[source as usize] {
                SourceHealth::Failing {
                    since_ms,
                    last_error,
                    ..
                } => Some(SourceFailure {
                    source: source.monitored(),
                    since_ms: *since_ms,
                    error: last_error.clone(),
                }),
                SourceHealth::Working => None,
            })
            .collect()
    }

    #[cfg(test)]
    pub fn of(&self, source: Source) -> &SourceHealth {
        &self.sources[source as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn broken() -> CollectError {
        CollectError::Pdh {
            call: "PdhCollectQueryData",
            status: 0xC000_0BBD,
        }
    }

    #[test]
    fn reopen_wait_doubles_up_to_a_cap() {
        let mut backoff = Backoff::new();
        let mut gaps = Vec::new();
        let mut since = 0;
        while gaps.len() < 9 {
            since += 1;
            if backoff.due() {
                gaps.push(since);
                since = 0;
                backoff.failed();
            }
        }

        assert_eq!(gaps, [1, 1, 2, 4, 8, 16, 32, 60, 60]);
    }

    #[test]
    fn source_health_tracks_failure_and_recovery() {
        let mut healths = SourceHealths::default();

        let announced = healths.record::<()>(Source::Pdh, 1_000, &Err(broken()));
        assert!(announced.is_some_and(|text| text.contains("0xc0000bbd")));
        let announced = healths.record::<()>(Source::Pdh, 2_000, &Err(broken()));
        assert_eq!(announced, None, "the same error is announced once");
        assert!(matches!(
            healths.of(Source::Pdh),
            SourceHealth::Failing {
                since_ms: 1_000,
                failures: 2,
                ..
            }
        ));
        assert_eq!(healths.of(Source::Cpu), &SourceHealth::Working);

        let announced = healths.record(Source::Pdh, 3_000, &Ok(7));
        assert_eq!(announced, None);
        assert_eq!(healths.of(Source::Pdh), &SourceHealth::Working);
    }

    #[test]
    fn the_failing_sources_are_listed_and_a_change_is_flagged_once() {
        let mut healths = SourceHealths::default();
        assert!(!healths.take_changed());

        healths.record::<()>(Source::Pdh, 1_000, &Err(broken()));
        assert!(healths.take_changed());
        assert!(!healths.take_changed(), "taking clears the flag");
        healths.record::<()>(Source::Pdh, 2_000, &Err(broken()));
        assert!(!healths.take_changed(), "the same error changes nothing");

        let failing = healths.failing();
        assert_eq!(failing.len(), 1);
        assert_eq!(failing[0].source, MonitorSource::Counters);
        assert_eq!(failing[0].since_ms, 1_000);

        healths.record(Source::Memory, 3_000, &Ok(1));
        assert!(
            !healths.take_changed(),
            "a source that always read is not news"
        );
        healths.record(Source::Pdh, 3_000, &Ok(1));
        assert!(healths.take_changed());
        assert!(healths.failing().is_empty());
    }

    #[test]
    fn a_different_error_is_announced_again() {
        let mut healths = SourceHealths::default();
        healths.record::<()>(Source::Memory, 1, &Err(broken()));

        let announced = healths.record::<()>(
            Source::Memory,
            2,
            &Err(CollectError::Pdh {
                call: "PdhCollectQueryData",
                status: 1,
            }),
        );

        assert!(announced.is_some());
    }
}
