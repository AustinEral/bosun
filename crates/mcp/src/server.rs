//! MCP server management (spawn, communicate, lifecycle).

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::error::{Error, Result};
use crate::protocol::{
    CallToolParams, CallToolResult, InitializeParams, InitializeResult, JsonRpcRequest,
    JsonRpcResponse, ListToolsResult, RequestId, Tool,
};

/// Default timeout for MCP operations.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum output size (1MB).
/// Sized for large tool outputs (file reads, search results).
pub const MAX_OUTPUT_SIZE: usize = 1024 * 1024;

/// Configuration for an MCP server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

/// Handle to a running MCP server.
pub struct Server {
    config: ServerConfig,
    process: Mutex<Child>,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    next_id: AtomicI64,
    initialized: Mutex<bool>,
    server_info: Mutex<Option<InitializeResult>>,
    tools: Mutex<Vec<Tool>>,
}

impl Server {
    /// Spawn a new MCP server process.
    pub async fn spawn(config: ServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .envs(&config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let mut process = cmd.spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| Error::Spawn(std::io::Error::other("failed to capture stdin")))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| Error::Spawn(std::io::Error::other("failed to capture stdout")))?;

        Ok(Self {
            config,
            process: Mutex::new(process),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            next_id: AtomicI64::new(1),
            initialized: Mutex::new(false),
            server_info: Mutex::new(None),
            tools: Mutex::new(Vec::new()),
        })
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Initialize the server (must be called before other operations).
    pub async fn initialize(&self) -> Result<&Self> {
        let params = InitializeParams::default();
        let result: InitializeResult = self.request("initialize", Some(params)).await?;

        // Send initialized notification
        self.notify("notifications/initialized", None::<()>).await?;

        // Store server info
        *self.server_info.lock().await = Some(result);
        *self.initialized.lock().await = true;

        // Fetch tools if the server supports them
        self.refresh_tools().await?;

        Ok(self)
    }

    /// Check if the server is initialized.
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.lock().await
    }

    /// Get server info (after initialization).
    pub async fn server_info(&self) -> Option<InitializeResult> {
        self.server_info.lock().await.clone()
    }

    /// Refresh the list of available tools.
    pub async fn refresh_tools(&self) -> Result<()> {
        let result: ListToolsResult = self.request("tools/list", None::<()>).await?;
        *self.tools.lock().await = result.tools;
        Ok(())
    }

    /// Get the list of available tools.
    pub async fn tools(&self) -> Vec<Tool> {
        self.tools.lock().await.clone()
    }

    /// Call a tool by name.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<CallToolResult> {
        if !*self.initialized.lock().await {
            return Err(Error::NotInitialized);
        }

        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let result: CallToolResult = self.request("tools/call", Some(params)).await?;

        // Check for error flag
        if result.is_error {
            let error_text = result
                .content
                .iter()
                .filter_map(|c| c.as_text())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(Error::ToolCallFailed(error_text));
        }

        Ok(result)
    }

    /// Check if the server process is still running.
    pub async fn is_running(&self) -> bool {
        let mut process = self.process.lock().await;
        matches!(process.try_wait(), Ok(None))
    }

    /// Shut down the server gracefully.
    pub async fn shutdown(self) -> Result<()> {
        // Send shutdown notification (best effort)
        let _ = self.notify("shutdown", None::<()>).await;

        // Kill the process
        let mut process = self.process.lock().await;
        let _ = process.kill().await;

        Ok(())
    }

    // --- Internal methods ---

    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.next_id.fetch_add(1, Ordering::SeqCst))
    }

    async fn request<P, R>(&self, method: &str, params: Option<P>) -> Result<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let id = self.next_request_id();
        let mut request = JsonRpcRequest::new(id.clone(), method);
        if let Some(p) = params {
            request = request.with_params(p);
        }

        // Send request
        let request_json = serde_json::to_string(&request)?;
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(request_json.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        // Read response with timeout
        let response = timeout(DEFAULT_TIMEOUT, self.read_response())
            .await
            .map_err(|_| Error::Timeout)??;

        // Verify response ID matches
        if response.id != id {
            return Err(Error::InvalidResponse(format!(
                "response ID mismatch: expected {id:?}, got {:?}",
                response.id
            )));
        }

        // Extract result
        let result_value = response.into_result()?;
        let result: R = serde_json::from_value(result_value)?;

        Ok(result)
    }

    async fn notify<P>(&self, method: &str, params: Option<P>) -> Result<()>
    where
        P: serde::Serialize,
    {
        // Notifications have no ID
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.and_then(|p| serde_json::to_value(p).ok())
        });

        let notification_json = serde_json::to_string(&notification)?;
        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(notification_json.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        Ok(())
    }

    async fn read_response(&self) -> Result<JsonRpcResponse> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();

        let bytes_read = stdout.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(Error::ServerExited);
        }

        // Check output size
        if line.len() > MAX_OUTPUT_SIZE {
            return Err(Error::OutputTooLarge {
                size: line.len(),
                max: MAX_OUTPUT_SIZE,
            });
        }

        let response: JsonRpcResponse = serde_json::from_str(&line)?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_config_creation() {
        let config = ServerConfig {
            name: "test".to_string(),
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
        };
        assert_eq!(config.name, "test");
    }
}
