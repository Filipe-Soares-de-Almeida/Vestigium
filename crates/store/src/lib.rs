//! Local SQLite history of Vestigium.

mod batch;
mod ring;
mod schema;
mod segment;

pub use batch::Batcher;
pub use ring::Ring;
pub use schema::{SCHEMA_VERSION, Store};
pub use segment::{Segment, Series, decode, encode};

/// Why a history operation failed.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("segment encoding error: {0}")]
    Codec(#[from] postcard::Error),
    #[error("unknown segment format {0}")]
    UnknownFormat(u8),
    #[error("empty segment")]
    EmptySegment,
    #[error("compressed segment is corrupt")]
    Corrupt,
    #[error("database schema {found} is newer than this version supports ({supported})")]
    NewerSchema { found: u32, supported: u32 },
    #[error("cannot create the data folder: {0}")]
    Io(#[from] std::io::Error),
}
