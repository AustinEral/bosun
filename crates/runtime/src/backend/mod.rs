//! LLM backend abstraction.
//!
//! Provides a trait for LLM backends, allowing Bosun to support multiple
//! providers (Anthropic, OpenAI, etc.) through a unified interface.
//!
//! ## Block-based Messages
//!
//! Messages use content blocks to support both text and tool interactions:
//! - `ContentBlock::Text` — plain text content
//! - `ContentBlock::ToolUse` — model requesting a tool call
//! - `ContentBlock::ToolResult` — result of a tool execution

mod anthropic;

pub use anthropic::{AnthropicAuth, AnthropicBackend};

use crate::Result;
use serde::{Deserialize, Serialize};
use std::future::Future;
use storage::Role;

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call (used to correlate results).
    pub id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// Arguments as JSON value.
    pub input: serde_json::Value,
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this result corresponds to.
    pub tool_use_id: String,
    /// Output content (typically stringified).
    pub content: String,
    /// Whether the tool execution failed.
    #[serde(default)]
    pub is_error: bool,
}

/// A content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text { text: String },
    /// Tool use request from the model.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result from execution.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

impl ContentBlock {
    /// Create a text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create a tool use block.
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Create a tool result block.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error,
        }
    }

    /// Extract text if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    /// Extract tool call if this is a tool use block.
    pub fn as_tool_use(&self) -> Option<ToolCall> {
        match self {
            Self::ToolUse { id, name, input } => Some(ToolCall {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            _ => None,
        }
    }
}

/// Reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopReason {
    /// Natural end of response.
    #[default]
    EndTurn,
    /// Model wants to use a tool.
    ToolUse,
    /// Hit max tokens limit.
    MaxTokens,
    /// Stopped by stop sequence.
    StopSequence,
}

impl StopReason {
    /// Parse from Anthropic API stop_reason string.
    pub fn from_anthropic(s: &str) -> Self {
        match s {
            "tool_use" => Self::ToolUse,
            "max_tokens" => Self::MaxTokens,
            "stop_sequence" => Self::StopSequence,
            _ => Self::EndTurn,
        }
    }

    /// Whether this indicates the model wants to use tools.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, Self::ToolUse)
    }
}

/// A message in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a user message with text content.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::text(content)],
        }
    }

    /// Create an assistant message with text content.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::text(content)],
        }
    }

    /// Create a system message with text content.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![ContentBlock::text(content)],
        }
    }

    /// Create a user message containing tool results.
    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Self {
            role: Role::User,
            content: results
                .into_iter()
                .map(|r| ContentBlock::tool_result(r.tool_use_id, r.content, r.is_error))
                .collect(),
        }
    }

    /// Create an assistant message with content blocks.
    pub fn assistant_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: blocks,
        }
    }

    /// Get combined text content (ignores tool blocks).
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool calls from this message.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.content
            .iter()
            .filter_map(|b| b.as_tool_use())
            .collect()
    }
}

/// Tool definition to expose to the model.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    /// Tool name (should match MCP tool name).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the input parameters.
    pub input_schema: serde_json::Value,
}

/// Request to send to an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatRequest<'a> {
    pub messages: &'a [Message],
    pub system: Option<&'a str>,
    /// Tools available for the model to use.
    pub tools: &'a [ToolDef],
}

impl<'a> ChatRequest<'a> {
    /// Create a simple request with no tools.
    pub fn simple(messages: &'a [Message], system: Option<&'a str>) -> Self {
        Self {
            messages,
            system,
            tools: &[],
        }
    }
}

/// Token usage information from an LLM response.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Usage {
    /// Tokens consumed by the input (prompt).
    pub input_tokens: u32,
    /// Tokens generated in the output (completion).
    pub output_tokens: u32,
}

/// Response from an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The content blocks in the response.
    pub content: Vec<ContentBlock>,
    /// Token usage statistics.
    pub usage: Usage,
    /// Why the model stopped generating.
    pub stop_reason: StopReason,
}

impl ChatResponse {
    /// Get combined text content (ignores tool blocks).
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| b.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Extract all tool calls from the response.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.content
            .iter()
            .filter_map(|b| b.as_tool_use())
            .collect()
    }

    /// Whether the model wants to use tools.
    pub fn has_tool_calls(&self) -> bool {
        self.stop_reason.is_tool_use() || self.content.iter().any(|b| b.as_tool_use().is_some())
    }
}

/// Trait for LLM backends.
///
/// Implementations handle the specifics of communicating with different
/// LLM providers (API calls, etc.).
pub trait LlmBackend: Send + Sync {
    /// Send a chat request and get a response.
    fn chat(&self, request: ChatRequest<'_>) -> impl Future<Output = Result<ChatResponse>> + Send;
}
