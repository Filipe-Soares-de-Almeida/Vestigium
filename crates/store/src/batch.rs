use std::time::Duration;

use vestigium_core::{MinuteRollup, MinuteSample, ProcessTop, Sample, minute_of};

use crate::segment::MAX_SEGMENT_SPAN_MS;
use crate::{Segment, Series, encode};

/// Most items in one segment: a 16-core sample with six disks is 385 bytes, so a chunk stays
/// under a few MiB, far below what a segment may inflate to.
const SEGMENT_ITEMS: usize = 2_048;

/// Buffers items until a batch is due, then turns them into one segment per series.
#[derive(Debug)]
pub struct Batcher {
    interval_ms: i64,
    samples: Vec<Sample>,
    processes: Vec<ProcessTop>,
    minutes: Vec<MinuteSample>,
    rollup: MinuteRollup,
}

impl Batcher {
    pub fn new(interval: Duration) -> Batcher {
        Batcher {
            interval_ms: i64::try_from(interval.as_millis()).unwrap_or(i64::MAX),
            samples: Vec::new(),
            processes: Vec::new(),
            minutes: Vec::new(),
            rollup: MinuteRollup::default(),
        }
    }

    /// Buffers a sample; the first one of a new wall-clock minute closes the previous minute.
    pub fn push_sample(&mut self, sample: Sample) {
        if self
            .rollup
            .minute_ms()
            .is_some_and(|minute| minute != minute_of(sample.ts_ms))
        {
            self.minutes.extend(self.rollup.finish());
        }
        self.rollup.push(&sample);
        self.samples.push(sample);
    }

    pub fn push_processes(&mut self, top: ProcessTop) {
        self.processes.push(top);
    }

    /// True once the oldest buffered item is at least one interval old, or the clock is behind it.
    pub fn due(&self, now_ms: i64) -> bool {
        let oldest = [
            self.samples.first().map(|s| s.ts_ms),
            self.processes.first().map(|p| p.ts_ms),
            self.minutes.first().map(|m| m.ts_ms),
        ]
        .into_iter()
        .flatten()
        .min();
        oldest.is_some_and(|ts| now_ms < ts || now_ms.saturating_sub(ts) >= self.interval_ms)
    }

    /// Empties the buffer into one segment per non-empty series; the open minute stays.
    pub fn take(&mut self) -> Vec<Segment> {
        let mut segments = Vec::with_capacity(3);
        push_segment(&mut segments, Series::Samples, &self.samples, |s| s.ts_ms);
        push_segment(&mut segments, Series::Processes, &self.processes, |p| {
            p.ts_ms
        });
        push_segment(&mut segments, Series::Minutes, &self.minutes, |m| m.ts_ms);
        self.samples.clear();
        self.processes.clear();
        self.minutes.clear();
        segments
    }

    /// Like `take`, and also closes the open minute: for a pause, a suspend or a stop.
    pub fn take_final(&mut self) -> Vec<Segment> {
        self.minutes.extend(self.rollup.finish());
        self.take()
    }
}

fn push_segment<T: serde::Serialize>(
    segments: &mut Vec<Segment>,
    series: Series,
    items: &[T],
    ts_ms: impl Fn(&T) -> i64,
) {
    for chunk in split(items, &ts_ms) {
        let stamps = chunk.iter().map(&ts_ms);
        let (Some(start_ms), Some(end_ms)) = (stamps.clone().min(), stamps.max()) else {
            continue;
        };
        segments.push(Segment {
            series,
            start_ms,
            end_ms,
            data: encode(chunk),
        });
    }
}

/// Cuts items into runs of at most `SEGMENT_ITEMS` whose stamps stay within one segment span.
fn split<T>(items: &[T], ts_ms: impl Fn(&T) -> i64) -> Vec<&[T]> {
    let mut runs = Vec::new();
    let (mut from, mut low, mut high) = (0, i64::MAX, i64::MIN);
    for (at, item) in items.iter().enumerate() {
        let ts = ts_ms(item);
        let (new_low, new_high) = (low.min(ts), high.max(ts));
        let full =
            at - from == SEGMENT_ITEMS || new_high.saturating_sub(new_low) > MAX_SEGMENT_SPAN_MS;
        if at > from && full {
            runs.push(&items[from..at]);
            (from, low, high) = (at, ts, ts);
        } else {
            (low, high) = (new_low, new_high);
        }
    }
    if from < items.len() {
        runs.push(&items[from..]);
    }
    runs
}
