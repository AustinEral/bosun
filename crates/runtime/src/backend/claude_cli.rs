//! Claude CLI backend.
//!
//! Uses the `claude` CLI as a text-only backend. Tools are not supported.

use super::{ChatRequest, ChatResponse, LlmBackend};
use crate::{Error, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

/// Default command to run the Claude CLI.
const DEFAULT_COMMAND: &str = "claude";

/// Claude CLI backend.
///
/// Invokes the `claude` CLI for each request. This is a text-only backend;
/// tool calls are not supported.
pub struct ClaudeCliBackend {
    command: String,
    model: Option<String>,
}

impl ClaudeCliBackend {
    /// Create a new CLI backend using the default command.
    pub fn new() -> Self {
        Self {
            command: DEFAULT_COMMAND.to_string(),
            model: None,
        }
    }

    /// Create a CLI backend with a custom command path.
    pub fn with_command(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            model: None,
        }
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl Default for ClaudeCliBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from Claude CLI JSON output.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CliResponse {
    result: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[async_trait]
impl LlmBackend for ClaudeCliBackend {
    async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse> {
        // Build the prompt from messages
        let prompt = build_prompt(request.messages, request.system);

        // Build command
        let mut cmd = Command::new(&self.command);
        cmd.arg("-p")
            .arg("--output-format")
            .arg("json")
            .arg("--dangerously-skip-permissions");

        if let Some(ref model) = self.model {
            cmd.arg("--model").arg(model);
        }

        // Add the prompt as the final argument
        cmd.arg(&prompt);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::Exec(format!("failed to run claude CLI: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Exec(format!(
                "claude CLI failed with status {}: {stderr}",
                output.status
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Try to parse as JSON first
        if let Ok(response) = serde_json::from_str::<CliResponse>(&stdout) {
            return Ok(ChatResponse {
                content: response.result,
            });
        }

        // Fall back to treating stdout as plain text
        Ok(ChatResponse {
            content: stdout.trim().to_string(),
        })
    }

    fn supports_tools(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "claude-cli"
    }
}

/// Build a prompt string from messages and optional system prompt.
fn build_prompt(messages: &[super::Message], system: Option<&str>) -> String {
    let mut parts = Vec::new();

    if let Some(sys) = system {
        parts.push(format!("[System]\n{sys}\n"));
    }

    for msg in messages {
        let role = match msg.role {
            storage::Role::User => "User",
            storage::Role::Assistant => "Assistant",
            storage::Role::System => "System",
        };
        parts.push(format!("[{role}]\n{}\n", msg.content));
    }

    parts.join("\n")
}
