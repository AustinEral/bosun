//! Core LLM types (provider-agnostic).
//!
//! These types represent the universal concepts shared across LLM providers.
//! Provider-specific details belong in adapter modules.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Role of a message participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this call (used to correlate results).
    pub id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// Arguments as JSON.
    pub input: Value,
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this result corresponds to.
    pub tool_call_id: String,
    /// Output as JSON (text is `Value::String`).
    pub output: Value,
    /// Whether the execution failed.
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful text result.
    pub fn success(tool_call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            output: Value::String(output.into()),
            is_error: false,
        }
    }

    /// Create an error result.
    pub fn error(tool_call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            output: Value::String(message.into()),
            is_error: true,
        }
    }
}

/// A part of a message's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    /// Plain text content.
    Text { text: String },
    /// Tool call from assistant.
    ToolCall(ToolCall),
    /// Tool result from user.
    ToolResult(ToolResult),
}

impl Part {
    /// Create a text part.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text { text: s.into() }
    }

    /// Extract text if this is a text part.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Extract tool call if this is a tool call part.
    pub fn as_tool_call(&self) -> Option<&ToolCall> {
        match self {
            Self::ToolCall(tc) => Some(tc),
            _ => None,
        }
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

impl Message {
    /// Create a user message with text.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            parts: vec![Part::text(text)],
        }
    }

    /// Create an assistant message with text.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            parts: vec![Part::text(text)],
        }
    }

    /// Create a system message.
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            parts: vec![Part::text(text)],
        }
    }

    /// Create a user message with tool results.
    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Self {
            role: Role::User,
            parts: results.into_iter().map(Part::ToolResult).collect(),
        }
    }

    /// Get combined text content.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| p.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool calls.
    pub fn tool_calls(&self) -> Vec<&ToolCall> {
        self.parts.iter().filter_map(|p| p.as_tool_call()).collect()
    }
}

/// Tool specification exposed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for input parameters.
    pub input_schema: Value,
}

/// How the model should choose tools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToolChoice {
    /// Model decides whether to use tools.
    #[default]
    Auto,
    /// Model cannot use tools.
    None,
    /// Model must use at least one tool.
    Required,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FinishReason {
    /// Natural end of response.
    #[default]
    Stop,
    /// Model wants to call tools.
    ToolCalls,
    /// Hit token limit.
    Length,
    /// Content filtered.
    ContentFilter,
    /// Unknown reason (forward compatibility).
    Unknown(String),
}

impl FinishReason {
    /// Whether this indicates tool calls are pending.
    pub fn is_tool_calls(&self) -> bool {
        matches!(self, Self::ToolCalls)
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_text_extraction() {
        let msg = Message::user("hello world");
        assert_eq!(msg.text(), "hello world");
    }

    #[test]
    fn tool_result_constructors() {
        let success = ToolResult::success("id1", "output");
        assert!(!success.is_error);
        assert_eq!(success.output, Value::String("output".into()));

        let error = ToolResult::error("id2", "failed");
        assert!(error.is_error);
    }

    #[test]
    fn part_extraction() {
        let text = Part::text("hello");
        assert_eq!(text.as_text(), Some("hello"));
        assert!(text.as_tool_call().is_none());
    }
}
