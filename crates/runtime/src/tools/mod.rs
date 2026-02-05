//! Tool execution and MCP integration.

pub mod errors;
mod mcp_client;

pub use errors::ToolError;
pub use mcp_client::{CallToolResult, McpClient, McpError, Tool};
