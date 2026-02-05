//! LLM backend abstraction.
//!
//! Provides a trait for LLM backends, allowing Bosun to support multiple
//! providers (Anthropic API, OpenAI, etc.) through a unified interface.

mod anthropic;

pub use anthropic::{AnthropicAuth, AnthropicBackend};

use crate::Result;
use serde::Deserialize;
use std::future::Future;
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

/// Token usage information from an LLM response.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Usage {
    /// Tokens consumed by the input (prompt).
    pub input_tokens: u32,
    /// Tokens generated in the output (completion).
    pub output_tokens: u32,
}

impl Usage {
    /// Total tokens used (input + output).
    pub fn total_tokens(self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Response from an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The generated content.
    pub content: String,
    /// Token usage statistics.
    pub usage: Usage,
}

/// Trait for LLM backends.
///
/// Implementations handle the specifics of communicating with different
/// LLM providers (API calls, etc.).
pub trait LlmBackend: Send + Sync {
    /// Send a chat request and get a response.
    fn chat(&self, request: ChatRequest<'_>) -> impl Future<Output = Result<ChatResponse>> + Send;
}
