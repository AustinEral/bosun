//! MCP error types.

use crate::protocol::JsonRpcError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to spawn server: {0}")]
    Spawn(#[from] std::io::Error),

    #[error("server not initialized")]
    NotInitialized,

    #[error("server exited unexpectedly")]
    ServerExited,

    #[error("timeout waiting for response")]
    Timeout,

    #[error("failed to serialize request: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("JSON-RPC error: {0}")]
    JsonRpc(#[from] JsonRpcError),

    #[error("tool not found: {0}")]
    ToolNotFound(String),

    #[error("tool call failed: {0}")]
    ToolCallFailed(String),

    #[error("output too large: {size} bytes (max {max})")]
    OutputTooLarge { size: usize, max: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
