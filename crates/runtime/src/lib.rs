//! Core runtime for session and run management.

pub mod backend;
mod error;
pub mod llm;
mod session;

pub use backend::{
    AnthropicBackend, ChatRequest, ChatResponse, ClaudeCliBackend, LlmBackend, Message,
};
pub use error::{Error, Result};
pub use session::Session;
