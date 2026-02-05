//! MCP-backed tool host.

use super::{McpClient, McpError, Tool, ToolCall, ToolError, ToolHost, ToolSpec};
use serde_json::{Map, Value};

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
            .filter_map(tool_to_spec)
            .collect();
        Ok(Self { client, specs })
    }
}

impl ToolHost for McpToolHost {
    fn specs(&self) -> &[ToolSpec] {
        &self.specs
    }

    async fn execute(&self, call: &ToolCall) -> Result<Value, ToolError> {
        let arguments = to_arguments(&call.input)?;
        let result = self
            .client
            .call_tool(&call.name, arguments)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        // Convert CallToolResult content to JSON value
        serde_json::to_value(&result.content)
            .map_err(|e| ToolError::Execution(format!("serialize result: {e}")))
    }
}

/// Convert JSON value to optional argument map.
fn to_arguments(input: &Value) -> Result<Option<Map<String, Value>>, ToolError> {
    match input {
        Value::Null => Ok(None),
        Value::Object(map) => Ok(Some(map.clone())),
        _ => Err(ToolError::InvalidInput(
            "tool input must be a JSON object".into(),
        )),
    }
}

/// Convert rmcp Tool to our ToolSpec.
fn tool_to_spec(tool: Tool) -> Option<ToolSpec> {
    let name = tool.name.to_string();
    let description = tool.description.unwrap_or_default().to_string();
    // input_schema is Arc<Map<String, Value>> - clone inner and wrap as Object
    let schema = Value::Object((*tool.input_schema).clone());
    Some(ToolSpec { name, description, schema })
}
