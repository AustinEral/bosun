//! CLI error types.

use std::path::PathBuf;
use thiserror::Error;

/// CLI errors.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking downstream code.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The database file does not exist.
    ///
    /// This typically means no session has been started yet.
    #[error("database not found at {path}. Run 'bosun chat' first")]
    DatabaseNotFound { path: PathBuf },

    /// No session was found matching the given prefix.
    #[error("no session found matching '{prefix}'")]
    SessionNotFound { prefix: String },

    /// Multiple sessions match the given prefix.
    ///
    /// The user should provide a longer prefix to disambiguate.
    #[error("multiple sessions match '{prefix}': {matches:?}")]
    AmbiguousSession {
        prefix: String,
        matches: Vec<String>,
    },

    /// Configuration is invalid or missing required fields.
    #[error("config error: {0}")]
    Config(String),

    /// An error occurred in the runtime layer.
    #[error(transparent)]
    Runtime(#[from] runtime::Error),

    /// An error occurred in the storage layer.
    #[error(transparent)]
    Storage(#[from] storage::Error),

    /// An error occurred in the policy layer.
    #[error(transparent)]
    Policy(#[from] policy::Error),

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
