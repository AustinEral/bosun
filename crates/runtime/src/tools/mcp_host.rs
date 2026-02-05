//! MCP-backed tool host.

use super::{McpClient, McpError, ToolArguments, ToolCall, ToolError, ToolHost, ToolSpec};
use serde_json::Value;

/// Tool host backed by an MCP server.
pub struct McpToolHost {
    client: McpClient,
    specs: Vec<ToolSpec>,
}

impl McpToolHost {
    /// Spawn MCP server and cache tool specs.
    pub async fn spawn(
        command: impl AsRef<str>,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, McpError> {
        let client = McpClient::spawn(command, args).await?;
        let specs = client
            .list_tools()
            .await?
            .into_iter()
            .map(ToolSpec::from)
            .collect();
        Ok(Self { client, specs })
    }
}

impl ToolHost for McpToolHost {
    fn specs(&self) -> &[ToolSpec] {
        &self.specs
    }

    async fn execute(&self, call: &ToolCall) -> Result<Value, ToolError> {
        let arguments = ToolArguments::try_from(call.input.clone())?;
        let result = self
            .client
            .call_tool(&call.name, arguments.0)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        serde_json::to_value(&result.content)
            .map_err(|e| ToolError::Execution(format!("serialize result: {e}")))
    }
}
