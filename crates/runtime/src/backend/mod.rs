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

/// Content of a tool result.
///
/// Claude supports both plain text and structured content blocks in tool results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Simple text output.
    Text(String),
    /// Structured content blocks (for rich output).
    Blocks(Vec<ToolResultBlock>),
}

impl ToolResultContent {
    /// Create text content.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    /// Create from blocks.
    pub fn blocks(blocks: Vec<ToolResultBlock>) -> Self {
        Self::Blocks(blocks)
    }

    /// Get as plain text (joins block text if structured).
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ToolResultBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

impl From<String> for ToolResultContent {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for ToolResultContent {
    fn from(s: &str) -> Self {
        Self::Text(s.to_string())
    }
}

/// A block within tool result content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultBlock {
    /// Text content.
    Text { text: String },
    /// Image content (base64 encoded).
    Image { source: ImageSource },
}

/// Image source for tool result blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this result corresponds to.
    pub tool_use_id: String,
    /// Output content.
    pub content: ToolResultContent,
    /// Whether the tool execution failed.
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful text result.
    pub fn success(tool_use_id: impl Into<String>, content: impl Into<ToolResultContent>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    /// Create an error result.
    pub fn error(tool_use_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: ToolResultContent::Text(message.into()),
            is_error: true,
        }
    }
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
        #[serde(flatten)]
        content: ToolResultContent,
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

    /// Create a tool result block from a ToolResult.
    pub fn from_tool_result(result: ToolResult) -> Self {
        Self::ToolResult {
            tool_use_id: result.tool_use_id,
            content: result.content,
            is_error: result.is_error,
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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    /// Model's context window was exceeded.
    ModelContextWindowExceeded,
    /// Unknown stop reason (forward compatibility).
    Unknown(String),
}

impl StopReason {
    /// Parse from Anthropic API stop_reason string.
    pub fn from_anthropic(s: &str) -> Self {
        match s {
            "end_turn" => Self::EndTurn,
            "tool_use" => Self::ToolUse,
            "max_tokens" => Self::MaxTokens,
            "stop_sequence" => Self::StopSequence,
            "model_context_window_exceeded" => Self::ModelContextWindowExceeded,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Whether this indicates the model wants to use tools.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, Self::ToolUse)
    }

    /// Whether this indicates an error/limit condition.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::MaxTokens | Self::ModelContextWindowExceeded)
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
    ///
    /// Note: Tool results must immediately follow an assistant message with tool_use,
    /// and tool_result blocks should come first in the message content.
    pub fn tool_results(results: Vec<ToolResult>) -> Self {
        Self {
            role: Role::User,
            content: results
                .into_iter()
                .map(ContentBlock::from_tool_result)
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

/// How the model should choose tools.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Model decides whether to use tools.
    #[default]
    Auto,
    /// Model must use at least one tool.
    Any,
    /// Model must use the specified tool.
    Tool { name: String },
    /// Model cannot use tools.
    None,
}

impl ToolChoice {
    /// Force a specific tool.
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool { name: name.into() }
    }
}

/// Configuration for tool usage in a request.
#[derive(Debug, Clone, Default)]
pub struct ToolConfig<'a> {
    /// Tools available for the model to use.
    pub tools: &'a [ToolDef],
    /// How the model should choose tools.
    pub tool_choice: ToolChoice,
    /// Disable parallel tool calls (force sequential).
    pub disable_parallel_tool_use: bool,
}

impl<'a> ToolConfig<'a> {
    /// Create config with tools and default settings.
    pub fn new(tools: &'a [ToolDef]) -> Self {
        Self {
            tools,
            tool_choice: ToolChoice::Auto,
            disable_parallel_tool_use: false,
        }
    }

    /// Set tool choice mode.
    pub fn with_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = choice;
        self
    }

    /// Disable parallel tool use.
    pub fn sequential(mut self) -> Self {
        self.disable_parallel_tool_use = true;
        self
    }
}

/// Request to send to an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatRequest<'a> {
    pub messages: &'a [Message],
    pub system: Option<&'a str>,
    /// Tool configuration (tools, choice mode, parallel settings).
    pub tool_config: Option<ToolConfig<'a>>,
}

impl<'a> ChatRequest<'a> {
    /// Create a simple request with no tools.
    pub fn simple(messages: &'a [Message], system: Option<&'a str>) -> Self {
        Self {
            messages,
            system,
            tool_config: None,
        }
    }

    /// Create a request with tools.
    pub fn with_tools(
        messages: &'a [Message],
        system: Option<&'a str>,
        tool_config: ToolConfig<'a>,
    ) -> Self {
        Self {
            messages,
            system,
            tool_config: Some(tool_config),
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
