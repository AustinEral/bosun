//! Core runtime for session and run management.

pub mod backend;
mod error;
pub mod llm;
mod session;

pub use backend::{
    AnthropicAuth, AnthropicBackend, ChatRequest, ChatResponse, LlmBackend, Message,
};
pub use error::{Error, Result};
pub use session::Session;
