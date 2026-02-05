//! Tool-related types.

use super::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// The result returned to the model after a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToolResult {
    Success {
        tool_call_id: String,
        output: Value,
    },
    Failure {
        tool_call_id: String,
        error: ToolError,
    },
}

/// A tool definition exposed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
}
