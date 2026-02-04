//! Anthropic API backend.

use super::{ChatRequest, ChatResponse, LlmBackend, Usage};
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
    usage: ApiUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: u32,
    output_tokens: u32,
}

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
            .map(|m| ApiMessage {
                role: Self::role_to_api_str(m.role),
                content: m.content.clone(),
            })
            .collect();

        let api_request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: api_messages,
            system: self.auth.build_system(request.system),
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

        let content = api_response
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        let usage = Usage {
            input_tokens: api_response.usage.input_tokens,
            output_tokens: api_response.usage.output_tokens,
        };

        Ok(ChatResponse { content, usage })
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
    fn usage_total_tokens() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(usage.total_tokens(), 150);
    }

    #[test]
    fn usage_cost_calculation() {
        let usage = Usage {
            input_tokens: 1_000_000, // 1M input tokens
            output_tokens: 500_000,  // 500K output tokens
        };
        // At $3/MTok input and $15/MTok output (Sonnet pricing)
        let cost = usage.cost_usd(3.0, 15.0);
        assert!((cost - 10.5).abs() < 0.001); // $3 + $7.50 = $10.50
    }
}
