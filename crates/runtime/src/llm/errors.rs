use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("capability denied: {0}")]
    CapabilityDenied(String),
    #[error("timeout after {0}ms")]
    Timeout(u64),
    #[error("execution failed: {0}")]
    Execution(String),
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("network: {0}")]
    Network(String),
    #[error("provider api: {0}")]
    Api(String),
    #[error("invalid provider response: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error("invalid state: {0}")]
    InvalidState(String),
}
