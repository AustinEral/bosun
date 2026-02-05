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

// Conversion from rmcp Tool
impl From<super::Tool> for ToolSpec {
    fn from(tool: super::Tool) -> Self {
        Self {
            name: tool.name.to_string(),
            description: tool.description.unwrap_or_default().to_string(),
            schema: serde_json::Value::Object((*tool.input_schema).clone()),
        }
    }
}

/// Wrapper for tool call arguments (MCP expects Option<Map>).
#[derive(Debug, Clone)]
pub struct ToolArguments(pub Option<serde_json::Map<String, serde_json::Value>>);

impl TryFrom<serde_json::Value> for ToolArguments {
    type Error = super::ToolError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Null => Ok(Self(None)),
            serde_json::Value::Object(map) => Ok(Self(Some(map))),
            _ => Err(super::ToolError::InvalidInput(
                "tool input must be a JSON object".into(),
            )),
        }
    }
}
