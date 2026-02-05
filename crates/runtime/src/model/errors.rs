use thiserror::Error;

/// Errors from LLM provider calls.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new variants
/// in future versions without breaking downstream code.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ModelError {
    /// A network error occurred during the API call.
    #[error("network: {0}")]
    Network(String),

    /// The LLM provider returned an error response.
    #[error("provider api: {0}")]
    Api(String),

    /// The provider response could not be parsed.
    #[error("invalid provider response: {0}")]
    InvalidResponse(String),
}
