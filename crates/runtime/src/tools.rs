//! Tool host for MCP server management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use mcp::{CallToolResult, Server, ServerConfig, Tool};
use policy::{CapabilityRequest, Decision, Policy};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{Error, Result};

/// A tool with its source server.
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    /// The tool definition from MCP.
    pub tool: Tool,
    /// Name of the MCP server providing this tool.
    pub server_name: String,
}

/// Manages MCP servers and provides unified tool access.
pub struct ToolHost {
    servers: RwLock<HashMap<String, Arc<Server>>>,
    tools: RwLock<HashMap<String, RegisteredTool>>,
    configs: Vec<ServerConfig>,
}

impl ToolHost {
    /// Create a new tool host with the given server configurations.
    pub fn new(configs: Vec<ServerConfig>) -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            tools: RwLock::new(HashMap::new()),
            configs,
        }
    }

    /// Create an empty tool host (no MCP servers).
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Initialize all configured MCP servers and discover tools.
    pub async fn initialize(&self) -> Result<()> {
        for config in &self.configs {
            if let Err(e) = self.spawn_server(config.clone()).await {
                // Log error but continue with other servers
                eprintln!("Warning: Failed to initialize MCP server {}: {e}", config.name);
            }
        }
        Ok(())
    }

    async fn spawn_server(&self, config: ServerConfig) -> Result<()> {
        let name = config.name.clone();
        let server = Server::spawn(config).await.map_err(|e| {
            Error::Tool(format!("failed to spawn MCP server {name}: {e}"))
        })?;

        server.initialize().await.map_err(|e| {
            Error::Tool(format!("failed to initialize MCP server {name}: {e}"))
        })?;

        // Discover tools from this server
        let server_tools = server.tools().await;
        let server = Arc::new(server);

        // Register server and tools
        {
            let mut servers = self.servers.write().await;
            servers.insert(name.clone(), server);
        }

        {
            let mut tools = self.tools.write().await;
            for tool in server_tools {
                let tool_name = tool.name.clone();
                tools.insert(
                    tool_name.clone(),
                    RegisteredTool {
                        tool,
                        server_name: name.clone(),
                    },
                );
            }
        }

        Ok(())
    }

    /// List all available tools.
    pub async fn list_tools(&self) -> Vec<RegisteredTool> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Get a tool by name.
    pub async fn get_tool(&self, name: &str) -> Option<RegisteredTool> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Call a tool with policy enforcement.
    pub async fn call_tool(
        &self,
        name: &str,
        params: Option<Value>,
        policy: &Policy,
    ) -> Result<CallToolResult> {
        // Find the tool
        let registered = self.get_tool(name).await.ok_or_else(|| {
            Error::Tool(format!("tool not found: {name}"))
        })?;

        // Check capability policy
        let capability = tool_capability(name, &params);
        match policy.check(&capability) {
            Decision::Allow => {}
            Decision::Deny { reason } => {
                return Err(Error::CapabilityDenied(reason));
            }
        }

        // Get the server
        let servers = self.servers.read().await;
        let server = servers.get(&registered.server_name).ok_or_else(|| {
            Error::Tool(format!("server not found: {}", registered.server_name))
        })?;

        // Call the tool
        let result = server.call_tool(name, params).await.map_err(|e| {
            Error::Tool(format!("tool call failed: {e}"))
        })?;

        Ok(result)
    }

    /// Call a tool with a custom timeout.
    pub async fn call_tool_with_timeout(
        &self,
        name: &str,
        params: Option<Value>,
        policy: &Policy,
        timeout: Duration,
    ) -> Result<CallToolResult> {
        tokio::time::timeout(timeout, self.call_tool(name, params, policy))
            .await
            .map_err(|_| Error::Tool(format!("tool {name} timed out after {timeout:?}")))?
    }

    /// Shutdown all MCP servers.
    pub async fn shutdown(&self) {
        let mut servers = self.servers.write().await;
        for (_name, server) in servers.drain() {
            // Just drop the server - it will clean up on drop
            drop(server);
        }
    }
}

/// Map a tool call to a capability request.
///
/// This is a simplified mapping. In practice, tools would declare their
/// required capabilities in their schema.
fn tool_capability(name: &str, _params: &Option<Value>) -> CapabilityRequest {
    // Default: treat tool calls as exec (most restrictive)
    // TODO: Parse tool schemas for declared capabilities
    CapabilityRequest::Exec {
        command: name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_host_has_no_tools() {
        let host = ToolHost::empty();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tools = rt.block_on(host.list_tools());
        assert!(tools.is_empty());
    }
}
