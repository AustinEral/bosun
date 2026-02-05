//! MCP (Model Context Protocol) client integration.
//!
//! This module provides MCP server management using the official rmcp SDK.
//!
//! # Example
//!
//! ```ignore
//! use runtime::McpClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = McpClient::spawn("mcp-filesystem", ["--root", "./workspace"]).await?;
//!
//! let tools = client.list_tools().await?;
//! for tool in &tools {
//!     println!("Tool: {}", tool.name);
//! }
//! # Ok(())
//! # }
//! ```

use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    service::RunningService,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use std::sync::Arc;
use tokio::process::Command;

/// Error type for MCP operations.
pub type McpError = Box<dyn std::error::Error + Send + Sync>;

// Re-export rmcp types for convenience
pub use rmcp::model::{CallToolResult, Tool};

/// An MCP client connected to a server process.
pub struct McpClient {
    service: Arc<RunningService<rmcp::service::RoleClient, ()>>,
}

impl McpClient {
    /// Spawn an MCP server and connect to it.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to run (e.g., "mcp-filesystem")
    /// * `args` - Arguments to pass to the command
    pub async fn spawn(
        command: impl AsRef<str>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, McpError> {
        let command_str = command.as_ref().to_string();
        let args_vec: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();

        let transport = TokioChildProcess::new(Command::new(&command_str).configure(|cmd| {
            for arg in &args_vec {
                cmd.arg(arg);
            }
        }))?;

        let service = ().serve(transport).await?;

        Ok(Self {
            service: Arc::new(service),
        })
    }

    /// List available tools from the server.
    pub async fn list_tools(&self) -> Result<Vec<Tool>, McpError> {
        let response = self.service.list_tools(Default::default()).await?;
        Ok(response.tools)
    }

    /// Call a tool with the given name and arguments.
    pub async fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, McpError> {
        let params = CallToolRequestParams {
            name: name.into().into(),
            arguments,
            meta: None,
            task: None,
        };

        let result = self.service.call_tool(params).await?;
        Ok(result)
    }
}
