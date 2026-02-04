//! Policy error types.

use thiserror::Error;

/// Policy errors.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking downstream code.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A capability request was denied by policy.
    #[error("capability denied: {0}")]
    Denied(String),

    /// The policy configuration is invalid.
    #[error("invalid policy: {0}")]
    Invalid(String),

    /// Failed to parse a policy file.
    #[error("failed to parse policy: {0}")]
    Parse(String),

    /// An I/O error occurred while reading policy.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
