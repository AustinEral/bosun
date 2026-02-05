//! Anthropic API backend.

use super::{ChatRequest, ChatResponse, ContentBlock, LlmBackend, StopReason, ToolDef, Usage};
use crate::{Error, Result};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use storage::Role;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

// Claude Code OAuth constants
const CLAUDE_CODE_VERSION: &str = "2.1.2";
const OAUTH_BETA_HEADER: &str = "claude-code-20250219,oauth-2025-04-20,fine-grained-tool-streaming-2025-05-14,interleaved-thinking-2025-05-14";
const OAUTH_SYSTEM_PREFIX: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// Authentication mode for Anthropic API.
///
/// Use `ApiKey` for standard API keys (`sk-ant-api01-...`).
/// Use `ClaudeCodeOauth` for OAuth tokens from Claude Code CLI (`sk-ant-oat-...`).
#[derive(Debug, Clone)]
pub enum AnthropicAuth {
    /// Standard API key authentication.
    ApiKey(String),
    /// Claude Code OAuth token authentication.
    ClaudeCodeOauth(String),
}

impl std::fmt::Display for AnthropicAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey(_) => write!(f, "api_key"),
            Self::ClaudeCodeOauth(_) => write!(f, "claude_code_oauth"),
        }
    }
}

impl AnthropicAuth {
    /// Apply authentication headers to a request.
    fn apply_headers(&self, req: RequestBuilder) -> RequestBuilder {
        match self {
            Self::ApiKey(key) => req.header("x-api-key", key),
            Self::ClaudeCodeOauth(token) => req
                .header("anthropic-dangerous-direct-browser-access", "true")
                .header("Authorization", format!("Bearer {token}"))
                .header("anthropic-beta", OAUTH_BETA_HEADER)
                .header(
                    "user-agent",
                    format!("claude-cli/{CLAUDE_CODE_VERSION} (external, cli)"),
                )
                .header("x-app", "cli"),
        }
    }

    /// Build the system prompt in the appropriate format.
    fn build_system(&self, system: Option<&str>) -> Option<SystemPrompt> {
        match self {
            Self::ApiKey(_) => system.map(|s| SystemPrompt::Simple(s.to_string())),
            Self::ClaudeCodeOauth(_) => {
                let mut blocks = vec![SystemBlock {
                    block_type: "text",
                    text: OAUTH_SYSTEM_PREFIX.to_string(),
                    cache_control: CacheControl {
                        control_type: "ephemeral",
                    },
                }];
                if let Some(s) = system {
                    blocks.push(SystemBlock {
                        block_type: "text",
                        text: s.to_string(),
                        cache_control: CacheControl {
                            control_type: "ephemeral",
                        },
                    });
                }
                Some(SystemPrompt::Blocks(blocks))
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// API Request Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<SystemPrompt>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
}

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

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: &'static str,
    content: ApiContent,
}

/// Message content can be a simple string or blocks.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ApiContent {
    Text(String),
    Blocks(Vec<ApiContentBlock>),
}

/// Content block for API requests.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApiContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
}

#[derive(Debug, Serialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

impl From<&ToolDef> for ApiTool {
    fn from(tool: &ToolDef) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// API Response Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ApiResponseBlock>,
    usage: Usage,
    stop_reason: Option<String>,
}

/// Content block from API response.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApiResponseBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

impl From<ApiResponseBlock> for ContentBlock {
    fn from(block: ApiResponseBlock) -> Self {
        match block {
            ApiResponseBlock::Text { text } => ContentBlock::Text { text },
            ApiResponseBlock::ToolUse { id, name, input } => {
                ContentBlock::ToolUse { id, name, input }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Backend Implementation
// ─────────────────────────────────────────────────────────────────────────────

/// Builder for creating an Anthropic backend.
#[derive(Debug, Clone)]
pub struct AnthropicBackendBuilder {
    auth: AnthropicAuth,
    model: String,
    max_tokens: u32,
}

impl AnthropicBackendBuilder {
    /// Create a new builder with authentication and model.
    pub fn new(auth: AnthropicAuth, model: impl Into<String>) -> Self {
        Self {
            auth,
            model: model.into(),
            max_tokens: 4096,
        }
    }

    /// Set the maximum tokens for responses.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Build the backend.
    pub fn build(self) -> AnthropicBackend {
        AnthropicBackend {
            client: reqwest::Client::new(),
            auth: self.auth,
            model: self.model,
            max_tokens: self.max_tokens,
        }
    }
}

/// Anthropic API backend.
pub struct AnthropicBackend {
    client: reqwest::Client,
    auth: AnthropicAuth,
    model: String,
    max_tokens: u32,
}

impl AnthropicBackend {
    /// Create a builder for the Anthropic backend.
    pub fn builder(auth: AnthropicAuth, model: impl Into<String>) -> AnthropicBackendBuilder {
        AnthropicBackendBuilder::new(auth, model)
    }

    fn role_to_api_str(role: Role) -> &'static str {
        match role {
            Role::User | Role::System => "user",
            Role::Assistant => "assistant",
        }
    }

    /// Convert our Message to API format.
    fn message_to_api(msg: &super::Message) -> ApiMessage {
        let role = Self::role_to_api_str(msg.role);

        // Simple case: single text block -> string content
        if msg.content.len() == 1 {
            if let Some(text) = msg.content[0].as_text() {
                return ApiMessage {
                    role,
                    content: ApiContent::Text(text.to_string()),
                };
            }
        }

        // Complex case: multiple blocks or non-text content
        let blocks: Vec<ApiContentBlock> = msg
            .content
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => ApiContentBlock::Text { text: text.clone() },
                ContentBlock::ToolUse { id, name, input } => ApiContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                },
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => ApiContentBlock::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    content: content.clone(),
                    is_error: *is_error,
                },
            })
            .collect();

        ApiMessage {
            role,
            content: ApiContent::Blocks(blocks),
        }
    }
}

impl std::fmt::Display for AnthropicBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anthropic({}, auth={})", self.model, self.auth)
    }
}

impl LlmBackend for AnthropicBackend {
    async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse> {
        let api_messages: Vec<ApiMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(Self::message_to_api)
            .collect();

        let tools: Vec<ApiTool> = request.tools.iter().map(ApiTool::from).collect();

        let api_request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: api_messages,
            system: self.auth.build_system(request.system),
            tools,
        };

        let req = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "application/json");

        let req = self.auth.apply_headers(req);

        let response = req
            .json(&api_request)
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

        let stop_reason = api_response
            .stop_reason
            .as_deref()
            .map(StopReason::from_anthropic)
            .unwrap_or_default();

        let content: Vec<ContentBlock> = api_response
            .content
            .into_iter()
            .map(ContentBlock::from)
            .collect();

        Ok(ChatResponse {
            content,
            usage: api_response.usage,
            stop_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_display() {
        let api = AnthropicAuth::ApiKey("test".into());
        let oauth = AnthropicAuth::ClaudeCodeOauth("test".into());
        assert_eq!(api.to_string(), "api_key");
        assert_eq!(oauth.to_string(), "claude_code_oauth");
    }

    #[test]
    fn stop_reason_parsing() {
        assert_eq!(StopReason::from_anthropic("end_turn"), StopReason::EndTurn);
        assert_eq!(StopReason::from_anthropic("tool_use"), StopReason::ToolUse);
        assert_eq!(
            StopReason::from_anthropic("max_tokens"),
            StopReason::MaxTokens
        );
        assert_eq!(
            StopReason::from_anthropic("stop_sequence"),
            StopReason::StopSequence
        );
        assert_eq!(StopReason::from_anthropic("unknown"), StopReason::EndTurn);
    }

    #[test]
    fn content_block_helpers() {
        let text = ContentBlock::text("hello");
        assert_eq!(text.as_text(), Some("hello"));
        assert!(text.as_tool_use().is_none());

        let tool = ContentBlock::tool_use("id1", "my_tool", serde_json::json!({"arg": 1}));
        assert!(tool.as_text().is_none());
        let call = tool.as_tool_use().unwrap();
        assert_eq!(call.name, "my_tool");
        assert_eq!(call.id, "id1");
    }
}
