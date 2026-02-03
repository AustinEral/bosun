//! LLM client for Claude API.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use storage::Role;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_MAX_TOKENS: u32 = 4096;

// OAuth tokens require Claude Code identity headers
const CLAUDE_CODE_VERSION: &str = "2.1.2";
const OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

// Required system prompt prefix for OAuth tokens
const OAUTH_SYSTEM_PREFIX: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

impl Message {
    /// Create a simple text message.
    pub fn text(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a message with content blocks (for tool results).
    pub fn blocks(role: Role, blocks: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Get text content if this is a simple text message.
    pub fn as_text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text(s) => Some(s),
            MessageContent::Blocks(_) => None,
        }
    }
}

/// Message content - either simple text or content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text { text: String },
    /// Tool use request from assistant.
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    /// Tool result from user.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl ContentBlock {
    /// Create a text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create a tool result block.
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: None,
        }
    }

    /// Create an error tool result block.
    pub fn tool_error(tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: error.into(),
            is_error: Some(true),
        }
    }
}

/// A tool definition for the Claude API.
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
}

impl From<&mcp::Tool> for ToolDefinition {
    fn from(tool: &mcp::Tool) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        }
    }
}

/// Response from the LLM.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Content blocks in the response.
    pub content: Vec<ContentBlock>,
    /// Stop reason.
    pub stop_reason: StopReason,
    /// Token usage.
    pub usage: Usage,
}

impl LlmResponse {
    /// Get all text content concatenated.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get tool use requests.
    pub fn tool_uses(&self) -> Vec<(&str, &str, &Value)> {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input))
                }
                _ => None,
            })
            .collect()
    }

    /// Check if response requests tool use.
    pub fn has_tool_use(&self) -> bool {
        self.stop_reason == StopReason::ToolUse
    }
}

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
    #[serde(other)]
    Unknown,
}

/// Token usage information.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
}

// --- Internal API types ---

/// Request to the Claude API.
#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<SystemPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
}

/// System prompt - either a simple string or array of blocks with cache control.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SystemPrompt {
    Simple(String),
    Blocks(Vec<SystemBlock>),
}

#[derive(Debug, Serialize)]
struct SystemBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    cache_control: CacheControl,
}

#[derive(Debug, Serialize)]
struct CacheControl {
    #[serde(rename = "type")]
    control_type: &'static str,
}

/// Message in API format.
#[derive(Debug, Serialize)]
struct ApiMessage {
    role: &'static str,
    content: ApiContent,
}

/// Content in API format.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ApiContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

/// Response from the Claude API.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    stop_reason: StopReason,
    #[serde(default)]
    usage: Usage,
}

/// Anthropic API client.
pub struct Client {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl Client {
    /// Create a new client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Create a client from the ANTHROPIC_API_KEY environment variable.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::Config("ANTHROPIC_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    fn is_oauth_token(&self) -> bool {
        self.api_key.contains("sk-ant-oat")
    }

    /// Send messages and get a response, optionally with tools.
    pub async fn send(
        &self,
        messages: &[Message],
        system: Option<&str>,
        tools: Option<&[ToolDefinition]>,
    ) -> Result<LlmResponse> {
        let api_messages: Vec<ApiMessage> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| ApiMessage {
                role: match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "user", // filtered above
                },
                content: match &m.content {
                    MessageContent::Text(s) => ApiContent::Text(s.clone()),
                    MessageContent::Blocks(blocks) => ApiContent::Blocks(blocks.clone()),
                },
            })
            .collect();

        // For OAuth tokens, use the required system prefix
        // Skip cache_control when tools are present to avoid beta feature conflicts
        let effective_system = if self.is_oauth_token() {
            let full_system = match system {
                Some(s) => format!("{}\n\n{}", OAUTH_SYSTEM_PREFIX, s),
                None => OAUTH_SYSTEM_PREFIX.to_string(),
            };
            Some(SystemPrompt::Simple(full_system))
        } else {
            system.map(|s| SystemPrompt::Simple(s.to_string()))
        };

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: DEFAULT_MAX_TOKENS,
            messages: api_messages,
            system: effective_system,
            tools: tools.map(|t| t.to_vec()),
        };

        let mut req = self
            .http
            .post(ANTHROPIC_API_URL)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "application/json");

        if self.is_oauth_token() {
            // OAuth token - use Bearer auth with Claude Code identity headers
            req = req
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("anthropic-beta", OAUTH_BETA_HEADER)
                .header("anthropic-dangerous-direct-browser-access", "true")
                .header(
                    "user-agent",
                    format!("claude-cli/{CLAUDE_CODE_VERSION} (external, cli)"),
                )
                .header("x-app", "cli");
        } else {
            // Standard API key
            req = req.header("x-api-key", &self.api_key);
        }

        let response = req
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api(format!("{status}: {body}")));
        }

        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| Error::Api(e.to_string()))?;

        Ok(LlmResponse {
            content: api_response.content,
            stop_reason: api_response.stop_reason,
            usage: api_response.usage,
        })
    }

    /// Simple text send (backwards compatibility).
    pub async fn send_text(
        &self,
        messages: &[Message],
        system: Option<&str>,
    ) -> Result<String> {
        let response = self.send(messages, system, None).await?;
        Ok(response.text())
    }
}
