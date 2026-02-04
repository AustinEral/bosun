//! Runtime error types.

use thiserror::Error;

/// Runtime errors.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking downstream code.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Configuration is invalid or missing required fields.
    #[error("config error: {0}")]
    Config(String),

    /// A network request failed.
    #[error("network error: {0}")]
    Network(String),

    /// The LLM API returned an error.
    #[error("API error: {0}")]
    Api(String),

    /// The requested session was not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// The operation is invalid for the current state.
    #[error("invalid state: {0}")]
    InvalidState(String),

    /// A capability was denied by policy.
    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    /// An error occurred in the storage layer.
    #[error(transparent)]
    Storage(#[from] storage::Error),

    /// An error occurred in the policy layer.
    #[error(transparent)]
    Policy(#[from] policy::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
