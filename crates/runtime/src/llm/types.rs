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

/// Outcome of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolOutcome {
    /// Tool executed successfully.
    Success { output: Value },
    /// Tool execution failed.
    Error { message: String },
}

impl ToolOutcome {
    /// Create a successful outcome with text output.
    pub fn success(output: impl Into<String>) -> Self {
        Self::Success {
            output: Value::String(output.into()),
        }
    }

    /// Create a successful outcome with JSON output.
    pub fn success_json(output: Value) -> Self {
        Self::Success { output }
    }

    /// Create an error outcome.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    /// Whether this is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

/// Result of a tool execution, paired with call ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this result corresponds to.
    pub tool_call_id: String,
    /// Outcome of the execution.
    pub outcome: ToolOutcome,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(tool_call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            outcome: ToolOutcome::success(output),
        }
    }

    /// Create an error result.
    pub fn error(tool_call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            outcome: ToolOutcome::error(message),
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
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

impl Message {
    /// Create a message with a role and text content.
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![Part::text(text)],
        }
    }

    /// Create a user message with text.
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, text)
    }

    /// Create an assistant message with text.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, text)
    }

    /// Create a system message.
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, text)
    }

    /// Create a user message with tool results.
    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Self {
            role: Role::User,
            parts: results.into_iter().map(Part::ToolResult).collect(),
        }
    }

    /// Create a message from parts.
    pub fn from_parts(role: Role, parts: Vec<Part>) -> Self {
        Self { role, parts }
    }

    /// Add a part to this message.
    pub fn with_part(mut self, part: Part) -> Self {
        self.parts.push(part);
        self
    }

    /// Get combined text content.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool calls.
    pub fn tool_calls(&self) -> Vec<&ToolCall> {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::ToolCall(tc) => Some(tc),
                _ => None,
            })
            .collect()
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
    /// Model cannot use tools (even if provided).
    None,
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
    fn message_builder() {
        let msg = Message::user("hello").with_part(Part::text(" world"));
        assert_eq!(msg.text(), "hello world");
    }

    #[test]
    fn tool_outcome_variants() {
        let success = ToolOutcome::success("done");
        assert!(!success.is_error());

        let error = ToolOutcome::error("failed");
        assert!(error.is_error());
    }

    #[test]
    fn tool_result_constructors() {
        let success = ToolResult::success("id1", "output");
        assert!(!success.outcome.is_error());

        let error = ToolResult::error("id2", "failed");
        assert!(error.outcome.is_error());
    }
}
