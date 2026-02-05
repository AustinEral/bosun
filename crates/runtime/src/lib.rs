//! Bosun runtime — session and LLM backend management.
//!
//! This crate provides the core runtime for managing AI agent sessions,
//! including LLM backend abstraction, MCP tool integration, and session
//! lifecycle management.

mod error;
mod session;

pub mod llm;
pub mod providers;
pub mod tools;

// Error types
pub use error::{Error, Result};

// Session management
pub use session::Session;

// Provider exports (legacy API)
pub use providers::{
    AnthropicAuth, AnthropicBackend, ChatRequest, ChatResponse, LlmBackend, Message, Usage,
};

// New LLM protocol types
pub use llm::{Backend, ModelError, ModelRequest, ModelResponse, Part, Role, ToolCall, ToolResult};

// Tool types
pub use tools::{CallToolResult, McpClient, McpError, Tool, ToolError};

// Re-export ToolSpec from llm
pub use llm::ToolSpec;
