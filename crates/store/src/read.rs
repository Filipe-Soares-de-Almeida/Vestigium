use serde::de::DeserializeOwned;
use vestigium_core::{
    MinuteSample, ProcessTop, ProcessTopV1, Sample, SampleV1, SampleV2, minute_of,
};

use crate::{Series, Store, StoreError, decode};

impl Store {
    /// Samples taken in `[from_ms, to_ms]`, oldest first, from every stored layout.
    pub fn samples(&self, from_ms: i64, to_ms: i64) -> Result<Vec<Sample>, StoreError> {
        let mut samples = self.items::<SampleV1, Sample>(Series::SamplesV1, from_ms, to_ms)?;
        samples.extend(self.items::<SampleV2, Sample>(Series::SamplesV2, from_ms, to_ms)?);
        samples.extend(self.items::<Sample, Sample>(Series::Samples, from_ms, to_ms)?);
        samples.retain(|s| (from_ms..=to_ms).contains(&s.ts_ms));
        samples.sort_by_key(|s| s.ts_ms);
        Ok(samples)
    }

    /// Process rankings taken in `[from_ms, to_ms]`, oldest first, from every stored layout.
    pub fn tops(&self, from_ms: i64, to_ms: i64) -> Result<Vec<ProcessTop>, StoreError> {
        let mut tops =
            self.items::<ProcessTopV1, ProcessTop>(Series::ProcessesV1, from_ms, to_ms)?;
        tops.extend(self.items::<ProcessTop, ProcessTop>(Series::Processes, from_ms, to_ms)?);
        tops.retain(|t| (from_ms..=to_ms).contains(&t.ts_ms));
        tops.sort_by_key(|t| t.ts_ms);
        Ok(tops)
    }

    /// Minutes that overlap `[from_ms, to_ms]`, oldest first; two summaries of one minute are merged.
    pub fn minutes(&self, from_ms: i64, to_ms: i64) -> Result<Vec<MinuteSample>, StoreError> {
        let from_ms = minute_of(from_ms);
        let mut minutes =
            self.items::<MinuteSample, MinuteSample>(Series::Minutes, from_ms, to_ms)?;
        minutes.retain(|m| (from_ms..=to_ms).contains(&m.ts_ms));
        minutes.sort_by_key(|m| m.ts_ms);
        let mut merged: Vec<MinuteSample> = Vec::with_capacity(minutes.len());
        for minute in minutes {
            match merged.last_mut() {
                Some(last) if last.ts_ms == minute.ts_ms => last.merge(&minute),
                _ => merged.push(minute),
            }
        }
        Ok(merged)
    }

    /// Decodes every segment of `series` overlapping the range as `Stored`, converted to `T`.
    fn items<Stored: DeserializeOwned + Into<T>, T>(
        &self,
        series: Series,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<T>, StoreError> {
        let mut items = Vec::new();
        for segment in self.segments(series, from_ms, to_ms)? {
            items.extend(decode::<Stored>(&segment.data)?.into_iter().map(Into::into));
        }
        Ok(items)
    }
}
