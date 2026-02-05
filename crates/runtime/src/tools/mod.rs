//! Tool execution and MCP integration.

mod empty;
pub mod errors;
mod mcp_client;
mod r#trait;
mod types;

pub use empty::EmptyToolHost;
pub use errors::ToolError;
pub use mcp_client::{CallToolResult, McpClient, McpError, Tool};
pub use r#trait::ToolHost;
pub use types::{ToolCall, ToolResult, ToolSpec};
