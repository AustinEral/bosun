//! Claude CLI backend.
//!
//! Uses the `claude` CLI for LLM requests. This backend invokes the CLI
//! as a subprocess, passing messages as a formatted prompt string.
//!
//! Note: While the Claude CLI itself supports tools, this backend wrapper
//! currently operates in text-only mode for simplicity. Tool support could
//! be added by parsing the JSON output for tool calls.

use super::{ChatRequest, ChatResponse, LlmBackend, Message};
use crate::{Error, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use storage::Role;
use tokio::process::Command;

const DEFAULT_COMMAND: &str = "claude";

/// Claude CLI backend.
///
/// Invokes the `claude` CLI for each request.
pub struct ClaudeCliBackend {
    command: String,
    model: String,
}

impl ClaudeCliBackend {
    /// Create a new CLI backend with the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            command: DEFAULT_COMMAND.to_string(),
            model: model.into(),
        }
    }

    /// Create a CLI backend with a custom command path.
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }
}

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
        let prompt = build_prompt(request.messages, request.system);

        let mut cmd = Command::new(&self.command);
        cmd.arg("-p")
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(&self.model)
            .arg("--dangerously-skip-permissions")
            .arg(&prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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

        if let Ok(response) = serde_json::from_str::<CliResponse>(&stdout) {
            return Ok(ChatResponse {
                content: response.result,
            });
        }

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

/// Formats conversation messages into a single prompt string for the CLI.
///
/// The CLI expects a single text input, so we format the message history
/// with role labels to preserve conversation context.
fn build_prompt(messages: &[Message], system: Option<&str>) -> String {
    let mut parts = Vec::new();

    if let Some(sys) = system {
        parts.push(format!("[System]\n{sys}\n"));
    }

    for msg in messages {
        let role = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };
        parts.push(format!("[{role}]\n{}\n", msg.content));
    }

    parts.join("\n")
}
