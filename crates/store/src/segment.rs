use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::StoreError;

/// Version of the BLOB layout, stored as its first byte.
const FORMAT: u8 = 1;

/// Which kind of item a segment holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Series {
    Samples = 1,
    Processes = 2,
}

/// One batch of items of one series, stored as one row.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub series: Series,
    pub start_ms: i64,
    pub end_ms: i64,
    pub data: Vec<u8>,
}

/// Packs items into a segment BLOB: the format byte, then postcard.
pub fn encode<T: Serialize>(items: &[T]) -> Vec<u8> {
    postcard::to_extend(items, vec![FORMAT]).expect("postcard serializes plain data into a Vec")
}

pub fn decode<T: DeserializeOwned>(data: &[u8]) -> Result<Vec<T>, StoreError> {
    match data.split_first() {
        None => Err(StoreError::EmptySegment),
        Some((&FORMAT, body)) => Ok(postcard::from_bytes(body)?),
        Some((&other, _)) => Err(StoreError::UnknownFormat(other)),
    }
}
