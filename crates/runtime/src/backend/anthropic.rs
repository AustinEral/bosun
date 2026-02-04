//! Anthropic API backend.

use super::{ChatRequest, ChatResponse, LlmBackend};
use crate::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use storage::Role;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_MAX_TOKENS: u32 = 4096;

// OAuth tokens require Claude Code identity headers
const CLAUDE_CODE_VERSION: &str = "2.1.2";
const OAUTH_BETA_HEADER: &str = "claude-code-20250219,oauth-2025-04-20,fine-grained-tool-streaming-2025-05-14,interleaved-thinking-2025-05-14";

// Required system prompt prefix for OAuth tokens
const OAUTH_SYSTEM_PREFIX: &str = "You are Claude Code, Anthropics official CLI for Claude.";

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<SystemPrompt>,
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
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

/// Anthropic API backend.
pub struct AnthropicBackend {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicBackend {
    /// Create a new backend with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Create a backend from the ANTHROPIC_API_KEY environment variable.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::Config("ANTHROPIC_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    fn is_oauth_token(&self) -> bool {
        self.api_key.contains("sk-ant-oat")
    }

    fn role_to_str(role: Role) -> &'static str {
        match role {
            Role::User | Role::System => "user",
            Role::Assistant => "assistant",
        }
    }
}

#[async_trait]
impl LlmBackend for AnthropicBackend {
    async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse> {
        let api_messages: Vec<ApiMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| ApiMessage {
                role: Self::role_to_str(m.role),
                content: m.content.clone(),
            })
            .collect();

        let effective_system = if self.is_oauth_token() {
            let mut blocks = vec![SystemBlock {
                block_type: "text",
                text: OAUTH_SYSTEM_PREFIX.to_string(),
                cache_control: CacheControl {
                    control_type: "ephemeral",
                },
            }];
            if let Some(s) = request.system {
                blocks.push(SystemBlock {
                    block_type: "text",
                    text: s.to_string(),
                    cache_control: CacheControl {
                        control_type: "ephemeral",
                    },
                });
            }
            Some(SystemPrompt::Blocks(blocks))
        } else {
            request.system.map(|s| SystemPrompt::Simple(s.to_string()))
        };

        let api_request = ApiRequest {
            model: self.model.clone(),
            max_tokens: DEFAULT_MAX_TOKENS,
            messages: api_messages,
            system: effective_system,
        };

        let mut req = self
            .http
            .post(ANTHROPIC_API_URL)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "application/json");

        if self.is_oauth_token() {
            // OAuth tokens (sk-ant-oat-*) require Claude Code identity headers.
            // These headers authenticate as a Claude Code client, which is required
            // for OAuth-based API access outside the standard API key flow.
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
            req = req.header("x-api-key", &self.api_key);
        }

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

        let content = api_response
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatResponse { content })
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "anthropic-api"
    }
}
