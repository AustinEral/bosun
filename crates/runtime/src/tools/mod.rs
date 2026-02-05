//! Tool execution and MCP integration.

pub mod errors;
pub mod host;
mod mcp_client;

pub use errors::ToolError;
pub use host::{EmptyToolHost, ToolHost};
pub use mcp_client::{CallToolResult, McpClient, McpError, Tool};
