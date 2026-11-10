use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::StoreError;

/// BLOB layouts, stored as the first byte: postcard, or postcard compressed with deflate.
const FORMAT_POSTCARD: u8 = 1;
const FORMAT_DEFLATE: u8 = 2;
const DEFLATE_LEVEL: u8 = 6;
/// Largest decompressed segment accepted, so a corrupt BLOB cannot ask for unbounded memory;
/// well above what a record the watcher writes can reach.
const MAX_INFLATED: usize = 64 << 20;

/// Which kind of item a segment holds; an item whose layout changes gets a new code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Series {
    /// `SampleV1`, from before memory total and GPU were recorded.
    SamplesV1 = 1,
    /// `ProcessTopV1`, from before per-process GPU was recorded.
    ProcessesV1 = 2,
    Samples = 3,
    Processes = 4,
    Minutes = 5,
}

/// One batch of items of one series, stored as one row.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub series: Series,
    pub start_ms: i64,
    pub end_ms: i64,
    pub data: Vec<u8>,
}

/// Packs items into a segment BLOB: the format byte, then deflate of the postcard bytes.
pub fn encode<T: Serialize>(items: &[T]) -> Vec<u8> {
    let raw = postcard::to_stdvec(items).expect("postcard serializes plain data into a Vec");
    let mut data = vec![FORMAT_DEFLATE];
    data.extend(miniz_oxide::deflate::compress_to_vec(&raw, DEFLATE_LEVEL));
    data
}

pub fn decode<T: DeserializeOwned>(data: &[u8]) -> Result<Vec<T>, StoreError> {
    match data.split_first() {
        None => Err(StoreError::EmptySegment),
        Some((&FORMAT_POSTCARD, body)) => Ok(postcard::from_bytes(body)?),
        Some((&FORMAT_DEFLATE, body)) => {
            let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(body, MAX_INFLATED)
                .map_err(|_| StoreError::Corrupt)?;
            Ok(postcard::from_bytes(&raw)?)
        }
        Some((&other, _)) => Err(StoreError::UnknownFormat(other)),
    }
}
