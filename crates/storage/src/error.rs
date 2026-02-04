//! Storage error types.

use thiserror::Error;

/// Storage errors.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking downstream code.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Database operation failed.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// JSON serialization/deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Row deserialization failed.
    #[error("row deserialization error: {0}")]
    RowDeserialization(#[from] serde_rusqlite::Error),

    /// Requested item was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Stored data is corrupted or malformed.
    ///
    /// This indicates data that was successfully read from the database
    /// but could not be parsed into the expected format.
    #[error("corrupted data in {table} (id: {id}): {reason}")]
    Corrupted {
        table: &'static str,
        id: String,
        reason: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
