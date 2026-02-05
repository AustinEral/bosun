use super::errors::ModelError;
use crate::tools::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;

/// The role of a message sender.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// The result the runtime returned from a tool call.
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

/// A part of a message, which can be text or a tool interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Part {
    Text(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
}

/// A message, consisting of a role and one or more parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

/// Everything needed for a model request.
#[derive(Debug, Clone)]
pub struct ModelRequest<'a> {
    pub messages: &'a [Message],
    pub tools: &'a [ToolSpec],
}

/// The response from a model.
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub message: Message,
}

/// Trait for LLM provider backends.
pub trait Backend: Send + Sync {
    fn call(
        &self,
        request: ModelRequest<'_>,
    ) -> impl Future<Output = Result<ModelResponse, ModelError>> + Send;
}
