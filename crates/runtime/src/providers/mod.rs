//! LLM provider adapters.
//!
//! Each provider implements the backend trait for its specific API.

mod anthropic;

pub use anthropic::{AnthropicAuth, AnthropicBackend};

// Legacy types used by anthropic adapter (to be migrated in future PR)
use crate::Result;
use serde::Deserialize;
use std::future::Future;
use storage::Role;

/// A message in the conversation (legacy).
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

/// Request to send to an LLM backend (legacy).
#[derive(Debug, Clone)]
pub struct ChatRequest<'a> {
    pub messages: &'a [Message],
    pub system: Option<&'a str>,
}

/// Token usage information (legacy).
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Response from an LLM backend (legacy).
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub usage: Usage,
}

/// Trait for LLM backends (legacy).
pub trait LlmBackend: Send + Sync {
    fn chat(&self, request: ChatRequest<'_>) -> impl Future<Output = Result<ChatResponse>> + Send;
}
