//! Bosun runtime — session and LLM backend management.
//!
//! This crate provides the core runtime for managing AI agent sessions,
//! including LLM backend abstraction, MCP tool integration, and session
//! lifecycle management.
//!
//! # Overview
//!
//! The runtime is organized around these concepts:
//!
//! - **Session**: A conversation context that manages messages, tracks events,
//!   and enforces capability policies.
//! - **LlmBackend**: A trait abstracting LLM providers (Anthropic, etc.).
//! - **McpClient**: A client for connecting to MCP tool servers.
//!
//! # Example
//!
//! ```ignore
//! use runtime::{AnthropicAuth, AnthropicBackend, Session};
//! use storage::EventStore;
//! use policy::Policy;
//!
//! # async fn example() -> runtime::Result<()> {
//! let auth = AnthropicAuth::ApiKey("sk-ant-api01-...".into());
//! let backend = AnthropicBackend::builder(auth, "claude-sonnet-4-20250514").build();
//! let store = EventStore::in_memory()?;
//! let policy = Policy::restrictive();
//!
//! let mut session = Session::new(store, backend, policy)?;
//! let response = session.chat("Hello!").await?;
//! println!("{response}");
//! # Ok(())
//! # }
//! ```

mod backend;
mod error;
pub mod llm;
mod mcp;
mod session;

// LLM core types (provider-agnostic)
// Full module available as `llm::` for types that overlap with current backend
pub use llm::{FinishReason, Part, ToolCall, ToolChoice, ToolResult, ToolSpec};

// LLM backend types (current, to be migrated)
pub use backend::{
    AnthropicAuth, AnthropicBackend, ChatRequest, ChatResponse, LlmBackend, Message, Usage,
};

// Error types
pub use error::{Error, Result};

// MCP client types
pub use mcp::{CallToolResult, McpClient, McpError, Tool};

// Session management
pub use session::Session;
