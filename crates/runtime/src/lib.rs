//! Bosun runtime — session and LLM backend management.
//!
//! This crate provides the core runtime for managing AI agent sessions,
//! including LLM backend abstraction, MCP tool integration, and session
//! lifecycle management.

mod error;
mod session;

pub mod model;
pub mod providers;
pub mod tools;

// Error types
pub use error::{Error, Result};

// Session management
pub use session::Session;

// Provider exports
pub use providers::{AnthropicAuth, AnthropicBackend, AnthropicBackendBuilder};

// Model protocol types
pub use model::{
    Backend, Message, ModelError, ModelRequest, ModelResponse, Part, Role, ToolCall, ToolResult,
    ToolSpec, Usage,
};

// Tool types
pub use tools::{CallToolResult, EmptyToolHost, McpClient, McpError, Tool, ToolError, ToolHost};
