//! Local SQLite history of Vestigium.

mod batch;
mod captures;
mod prune;
mod read;
mod records;
mod schema;
mod segment;

pub use batch::Batcher;
pub use captures::CaptureSummary;
pub use prune::PruneReport;
pub use schema::{Batch, SCHEMA_VERSION, Store};
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
    #[error("no history database yet")]
    NotFound,
    #[error("database schema {found} differs from the one this version reads ({supported})")]
    SchemaMismatch { found: u32, supported: u32 },
    #[error("unknown {what} code {code}")]
    UnknownCode { what: &'static str, code: i64 },
    #[error("invalid setting: {0}")]
    Setting(#[from] vestigium_core::SettingError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
