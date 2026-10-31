use std::time::Duration;

use argos_core::{ProcessTop, Sample};

use crate::{Segment, Series, encode};

/// Buffers items until a batch is due, then turns them into one segment per series.
#[derive(Debug)]
pub struct Batcher {
    interval_ms: i64,
    samples: Vec<Sample>,
    processes: Vec<ProcessTop>,
}

impl Batcher {
    pub fn new(interval: Duration) -> Batcher {
        Batcher {
            interval_ms: i64::try_from(interval.as_millis()).unwrap_or(i64::MAX),
            samples: Vec::new(),
            processes: Vec::new(),
        }
    }

    pub fn push_sample(&mut self, sample: Sample) {
        self.samples.push(sample);
    }

    pub fn push_processes(&mut self, top: ProcessTop) {
        self.processes.push(top);
    }

    /// True once the oldest buffered item is at least one interval old.
    pub fn due(&self, now_ms: i64) -> bool {
        let oldest = [
            self.samples.first().map(|s| s.ts_ms),
            self.processes.first().map(|p| p.ts_ms),
        ]
        .into_iter()
        .flatten()
        .min();
        oldest.is_some_and(|ts| now_ms.saturating_sub(ts) >= self.interval_ms)
    }

    /// Empties the buffer into one segment per non-empty series.
    pub fn take(&mut self) -> Vec<Segment> {
        let mut segments = Vec::with_capacity(2);
        if let (Some(first), Some(last)) = (self.samples.first(), self.samples.last()) {
            segments.push(Segment {
                series: Series::Samples,
                start_ms: first.ts_ms,
                end_ms: last.ts_ms,
                data: encode(&self.samples),
            });
        }
        if let (Some(first), Some(last)) = (self.processes.first(), self.processes.last()) {
            segments.push(Segment {
                series: Series::Processes,
                start_ms: first.ts_ms,
                end_ms: last.ts_ms,
                data: encode(&self.processes),
            });
        }
        self.samples.clear();
        self.processes.clear();
        segments
    }
}
