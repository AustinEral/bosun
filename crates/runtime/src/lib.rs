//! Bosun runtime — session management and LLM backend abstraction.
//!
//! This crate provides the core runtime for Bosun, an AI agent runtime.
//! It handles conversation sessions, LLM interactions, and tool execution.
//!
//! # Overview
//!
//! The runtime crate provides:
//!
//! - **Session management** — Conversation state, message history, and event logging
//! - **LLM backends** — Trait-based abstraction over language model providers
//! - **Tool execution** — MCP-compatible tool host for extending agent capabilities
//! - **Policy enforcement** — Capability checks at the tool execution boundary
//!
//! # Core Concepts
//!
//! ## Session
//!
//! A [`Session`] represents a single conversation. It:
//! - Maintains message history
//! - Tracks token usage
//! - Logs events to storage
//! - Enforces capability policies
//!
//! ## Backend
//!
//! The [`Backend`] trait abstracts over LLM providers. Implementations handle
//! the specifics of API communication while exposing a uniform interface.
//! Currently available: [`AnthropicBackend`].
//!
//! ## ToolHost
//!
//! The [`ToolHost`] trait defines how tools are discovered and executed.
//! Tools extend the agent's capabilities (file access, web requests, etc.).
//! The runtime provides [`EmptyToolHost`] (no tools) and [`McpToolHost`]
//! (MCP protocol support).
//!
//! # Example
//!
//! ```rust,ignore
//! use runtime::{Session, AnthropicBackend, AnthropicAuth, EmptyToolHost};
//! use storage::EventStore;
//! use policy::Policy;
//!
//! // Set up components
//! let store = EventStore::open("agent.db")?;
//! let backend = AnthropicBackend::builder()
//!     .auth(AnthropicAuth::from_env()?)
//!     .build()?;
//! let policy = Policy::default();
//!
//! // Create session and chat
//! let mut session = Session::new(store, backend, policy)?;
//! let (response, usage) = session.chat("Hello!").await?;
//! println!("Response: {response}");
//! println!("Tokens: {} in, {} out", usage.input_tokens, usage.output_tokens);
//! ```
//!
//! # Re-exports
//!
//! This crate re-exports key types for convenience:
//!
//! - **Error handling:** [`Error`], [`Result`]
//! - **Session:** [`Session`]
//! - **Model types:** [`Backend`], [`Message`], [`Part`], [`Role`], [`Usage`],
//!   [`ModelRequest`], [`ModelResponse`], [`ModelError`]
//! - **Backend implementations:** [`AnthropicBackend`], [`AnthropicBackendBuilder`],
//!   [`AnthropicAuth`]
//! - **Tool types:** [`ToolHost`], [`ToolSpec`], [`ToolCall`], [`ToolResult`],
//!   [`ToolError`], [`ToolArguments`]
//! - **Tool implementations:** [`EmptyToolHost`], [`McpToolHost`], [`McpClient`],
//!   [`McpError`], [`Tool`], [`CallToolResult`]

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
    CallToolResult, EmptyToolHost, McpClient, McpError, McpToolHost, Tool, ToolArguments, ToolCall,
    ToolError, ToolHost, ToolResult, ToolSpec,
};
