//! LLM client for Claude API.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
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
    pub content: String,
}

impl Message {
    /// Create a text message.
    pub fn text(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

/// Response from the LLM.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Response text.
    pub text: String,
    /// Token usage.
    pub usage: Usage,
}

impl LlmResponse {
    /// Get the response text.
    pub fn text(&self) -> &str {
        &self.text
    }
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
    system: Option<String>,
}

/// Message in API format.
#[derive(Debug, Serialize)]
struct ApiMessage {
    role: &'static str,
    content: String,
}

/// Response from the Claude API.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
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

    /// Send messages and get a response.
    pub async fn send(
        &self,
        messages: &[Message],
        system: Option<&str>,
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
                content: m.content.clone(),
            })
            .collect();

        // For OAuth tokens, prepend the required system prefix
        let effective_system = if self.is_oauth_token() {
            let full_system = match system {
                Some(s) => format!("{}\n\n{}", OAUTH_SYSTEM_PREFIX, s),
                None => OAUTH_SYSTEM_PREFIX.to_string(),
            };
            Some(full_system)
        } else {
            system.map(|s| s.to_string())
        };

        let request = ApiRequest {
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
            // OAuth token - use Bearer auth with Claude Code identity headers
            let api_key = &self.api_key;
            req = req
                .header("Authorization", format!("Bearer {api_key}"))
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

        let text = api_response
            .content
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            text,
            usage: api_response.usage,
        })
    }
}
