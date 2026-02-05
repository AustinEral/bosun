use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during tool execution.
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
