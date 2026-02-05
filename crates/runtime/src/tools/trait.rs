//! Tool host trait.

use crate::model::{ToolCall, ToolSpec};
use crate::tools::ToolError;
use serde_json::Value;
use std::future::Future;

/// Trait for tool execution hosts.
///
/// Implementations provide tool specifications and execute tool calls.
/// This is the boundary between the model loop and side effects.
pub trait ToolHost: Send + Sync {
    /// Get available tool specifications.
    fn specs(&self) -> &[ToolSpec];

    /// Execute a tool call.
    fn execute(&self, call: &ToolCall) -> impl Future<Output = Result<Value, ToolError>> + Send;
}
