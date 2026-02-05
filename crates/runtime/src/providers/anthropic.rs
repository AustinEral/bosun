//! Anthropic API backend.

use crate::model::{
    Backend, Message, ModelError, ModelRequest, ModelResponse, Part, Role, ToolCall, ToolResult,
    ToolSpec, Usage,
};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

// Claude Code OAuth constants
const CLAUDE_CODE_VERSION: &str = "2.1.2";
const OAUTH_BETA_HEADER: &str = "claude-code-20250219,oauth-2025-04-20,fine-grained-tool-streaming-2025-05-14,interleaved-thinking-2025-05-14";
const OAUTH_SYSTEM_PREFIX: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// Authentication mode for Anthropic API.
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

    fn build_system(&self, system: Option<&str>) -> Option<ApiSystemPrompt> {
        match self {
            Self::ApiKey(_) => system.map(|s| ApiSystemPrompt::Simple(s.to_string())),
            Self::ClaudeCodeOauth(_) => {
                let mut blocks = vec![ApiSystemBlock {
                    block_type: "text",
                    text: OAUTH_SYSTEM_PREFIX.to_string(),
                    cache_control: ApiCacheControl {
                        control_type: "ephemeral",
                    },
                }];
                if let Some(s) = system {
                    blocks.push(ApiSystemBlock {
                        block_type: "text",
                        text: s.to_string(),
                        cache_control: ApiCacheControl {
                            control_type: "ephemeral",
                        },
                    });
                }
                Some(ApiSystemPrompt::Blocks(blocks))
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// API Wire Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<ApiSystemPrompt>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ApiSystemPrompt {
    Simple(String),
    Blocks(Vec<ApiSystemBlock>),
}

#[derive(Debug, Serialize)]
struct ApiSystemBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    cache_control: ApiCacheControl,
}

#[derive(Debug, Serialize)]
struct ApiCacheControl {
    #[serde(rename = "type")]
    control_type: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: &'static str,
    content: ApiContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ApiContent {
    Text(String),
    Blocks(Vec<ApiContentBlock>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApiContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
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
    input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ApiResponseBlock>,
    usage: ApiUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApiResponseBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: u32,
    output_tokens: u32,
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
    system: Option<String>,
}

impl AnthropicBackendBuilder {
    pub fn new(auth: AnthropicAuth, model: impl Into<String>) -> Self {
        Self {
            auth,
            model: model.into(),
            max_tokens: 4096,
            system: None,
        }
    }

    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn build(self) -> AnthropicBackend {
        AnthropicBackend {
            client: reqwest::Client::new(),
            auth: self.auth,
            model: self.model,
            max_tokens: self.max_tokens,
            system: self.system,
        }
    }
}

/// Anthropic API backend.
pub struct AnthropicBackend {
    client: reqwest::Client,
    auth: AnthropicAuth,
    model: String,
    max_tokens: u32,
    system: Option<String>,
}

impl AnthropicBackend {
    pub fn builder(auth: AnthropicAuth, model: impl Into<String>) -> AnthropicBackendBuilder {
        AnthropicBackendBuilder::new(auth, model)
    }

    fn role_to_api(role: Role) -> &'static str {
        match role {
            Role::User | Role::System => "user",
            Role::Assistant => "assistant",
        }
    }

    fn message_to_api(msg: &Message) -> ApiMessage {
        let role = Self::role_to_api(msg.role);

        // Simple case: single text part
        if msg.parts.len() == 1 {
            if let Part::Text(text) = &msg.parts[0] {
                return ApiMessage {
                    role,
                    content: ApiContent::Text(text.clone()),
                };
            }
        }

        // Complex case: multiple parts or non-text
        let blocks: Vec<ApiContentBlock> = msg
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::Text(text) => Some(ApiContentBlock::Text { text: text.clone() }),
                Part::ToolCall(call) => Some(ApiContentBlock::ToolUse {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    input: call.input.clone(),
                }),
                Part::ToolResult(result) => {
                    let (tool_use_id, content, is_error) = match result {
                        ToolResult::Success {
                            tool_call_id,
                            output,
                        } => (tool_call_id.clone(), output.to_string(), false),
                        ToolResult::Failure {
                            tool_call_id,
                            error,
                        } => (tool_call_id.clone(), error.to_string(), true),
                    };
                    Some(ApiContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    })
                }
            })
            .collect();

        ApiMessage {
            role,
            content: ApiContent::Blocks(blocks),
        }
    }

    fn tool_to_api(spec: &ToolSpec) -> ApiTool {
        ApiTool {
            name: spec.name.clone(),
            description: spec.description.clone(),
            input_schema: spec.schema.clone(),
        }
    }

    fn response_to_message(blocks: Vec<ApiResponseBlock>) -> Message {
        let parts: Vec<Part> = blocks
            .into_iter()
            .filter_map(|block| match block {
                ApiResponseBlock::Text { text } => Some(Part::Text(text)),
                ApiResponseBlock::ToolUse { id, name, input } => {
                    Some(Part::ToolCall(ToolCall { id, name, input }))
                }
                ApiResponseBlock::Unknown => None,
            })
            .collect();

        Message {
            role: Role::Assistant,
            parts,
        }
    }
}

impl std::fmt::Display for AnthropicBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "anthropic({}, auth={})", self.model, self.auth)
    }
}

impl Backend for AnthropicBackend {
    async fn call(&self, request: ModelRequest<'_>) -> Result<ModelResponse, ModelError> {
        let api_messages: Vec<ApiMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(Self::message_to_api)
            .collect();

        let tools: Vec<ApiTool> = request.tools.iter().map(Self::tool_to_api).collect();

        let api_request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: api_messages,
            system: self.auth.build_system(self.system.as_deref()),
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
            .map_err(|e| ModelError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ModelError::Api(format!("{status}: {body}")));
        }

        let api_response: ApiResponse = response
            .json()
            .await
            .map_err(|e| ModelError::InvalidResponse(e.to_string()))?;

        let message = Self::response_to_message(api_response.content);
        let usage = Usage {
            input_tokens: api_response.usage.input_tokens,
            output_tokens: api_response.usage.output_tokens,
        };

        Ok(ModelResponse { message, usage })
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
}
