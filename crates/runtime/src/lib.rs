//! Bosun runtime — session and LLM backend management.

mod error;
mod session;

pub mod model;
pub mod tools;

// Error types
pub use error::{Error, Result};

// Session
pub use session::Session;

// Model types
pub use model::{
    AnthropicAuth, AnthropicBackend, AnthropicBackendBuilder, Backend, Message, ModelError,
    ModelRequest, ModelResponse, Part, Role, Usage,
};

// Tool types
pub use tools::{
    CallToolResult, EmptyToolHost, McpClient, McpError, McpToolHost, Tool, ToolArguments,
    ToolCall, ToolError, ToolHost, ToolResult, ToolSpec,
};
