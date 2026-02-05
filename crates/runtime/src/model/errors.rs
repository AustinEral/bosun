use thiserror::Error;

/// Errors from LLM provider calls.
#[derive(Debug, Error)]
pub enum ModelError {
    #[error("network: {0}")]
    Network(String),
    #[error("provider api: {0}")]
    Api(String),
    #[error("invalid provider response: {0}")]
    InvalidResponse(String),
}
