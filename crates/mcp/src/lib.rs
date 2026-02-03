//! MCP (Model Context Protocol) client library.
//!
//! This crate provides a client for communicating with MCP servers via stdio.
//!
//! # Example
//!
//! ```no_run
//! use mcp::{Server, ServerConfig};
//! use std::collections::HashMap;
//!
//! # async fn example() -> mcp::Result<()> {
//! let config = ServerConfig {
//!     name: "filesystem".to_string(),
//!     command: "mcp-filesystem".to_string(),
//!     args: vec!["--root".to_string(), "./workspace".to_string()],
//!     env: HashMap::new(),
//! };
//!
//! let server = Server::spawn(config).await?;
//! server.initialize().await?;
//!
//! let tools = server.tools().await;
//! for tool in tools {
//!     println!("Tool: {}", tool.name);
//! }
//!
//! let result = server.call_tool("read_file", Some(serde_json::json!({
//!     "path": "./README.md"
//! }))).await?;
//!
//! server.shutdown().await?;
//! # Ok(())
//! # }
//! ```

mod error;
mod protocol;
mod server;

pub use error::{Error, Result};
pub use protocol::{
    CallToolParams, CallToolResult, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcRequest, JsonRpcResponse, ListToolsResult, RequestId, ServerCapabilities, ServerInfo,
    Tool, ToolContent,
};
pub use server::{DEFAULT_TIMEOUT, MAX_OUTPUT_SIZE, Server, ServerConfig};
