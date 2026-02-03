//! CLI error types.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database not found at {path}. Run 'bosun chat' first")]
    DatabaseNotFound { path: PathBuf },

    #[error("no session found matching '{prefix}'")]
    SessionNotFound { prefix: String },

    #[error("multiple sessions match '{prefix}': {matches:?}")]
    AmbiguousSession {
        prefix: String,
        matches: Vec<String>,
    },

    #[error("ANTHROPIC_API_KEY not set")]
    MissingApiKey,

    #[error(transparent)]
    Runtime(#[from] runtime::Error),

    #[error(transparent)]
    Storage(#[from] storage::Error),

    #[error(transparent)]
    Policy(#[from] policy::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
