//! LLM backend abstraction.
//!
//! Provides a trait for LLM backends, allowing Bosun to support multiple
//! providers (Anthropic API, Claude CLI, OpenAI, etc.) through a unified interface.

mod anthropic;
mod claude_cli;

pub use anthropic::AnthropicBackend;
pub use claude_cli::ClaudeCliBackend;

use crate::Result;
use async_trait::async_trait;
use storage::Role;

/// A message in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// Request to send to an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatRequest<'a> {
    pub messages: &'a [Message],
    pub system: Option<&'a str>,
}

/// Response from an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
}

/// Trait for LLM backends.
///
/// Implementations handle the specifics of communicating with different
/// LLM providers (API calls, CLI invocations, etc.).
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Send a chat request and get a response.
    async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse>;

    /// Returns true if this backend supports tool calls.
    ///
    /// CLI backends typically return false (text-only).
    fn supports_tools(&self) -> bool {
        true
    }

    /// Human-readable name for this backend.
    fn name(&self) -> &str;
}
