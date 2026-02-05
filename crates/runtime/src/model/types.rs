use super::errors::ModelError;
use crate::tools::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;

/// The role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl Message {
    /// Get combined text content from all text parts.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool calls from this message.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::ToolCall(call) => Some(call.clone()),
                _ => None,
            })
            .collect()
    }
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: Value,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
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
    pub usage: Usage,
}

/// Trait for LLM provider backends.
pub trait Backend: Send + Sync {
    fn call(
        &self,
        request: ModelRequest<'_>,
    ) -> impl Future<Output = Result<ModelResponse, ModelError>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_text_extraction() {
        let msg = Message {
            role: Role::Assistant,
            parts: vec![
                Part::Text("Hello ".into()),
                Part::ToolCall(ToolCall {
                    id: "1".into(),
                    name: "test".into(),
                    input: Value::Null,
                }),
                Part::Text("world".into()),
            ],
        };
        assert_eq!(msg.text(), "Hello world");
    }

    #[test]
    fn message_tool_calls_extraction() {
        let msg = Message {
            role: Role::Assistant,
            parts: vec![
                Part::Text("Let me help".into()),
                Part::ToolCall(ToolCall {
                    id: "1".into(),
                    name: "search".into(),
                    input: Value::String("query".into()),
                }),
                Part::ToolCall(ToolCall {
                    id: "2".into(),
                    name: "read".into(),
                    input: Value::String("file".into()),
                }),
            ],
        };
        let calls = msg.tool_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "search");
        assert_eq!(calls[1].name, "read");
    }
}
