//! Session management.

use std::sync::Arc;

use crate::llm::{Client, Message};
use crate::tools::{RegisteredTool, ToolHost};
use crate::{Error, Result};
use policy::{CapabilityRequest, Decision, Policy};
use serde_json::Value;
use storage::{Event, EventKind, EventStore, Role, SessionId};

/// Maximum tool iterations per run to prevent infinite loops.
const MAX_TOOL_ITERATIONS: usize = 10;

/// Tool call pattern for runtime-based tool execution.
const TOOL_CALL_START: &str = "<tool_call>";
const TOOL_CALL_END: &str = "</tool_call>";

/// A conversation session with tool support.
pub struct Session {
    pub id: SessionId,
    store: EventStore,
    client: Client,
    policy: Policy,
    tool_host: Arc<ToolHost>,
    messages: Vec<Message>,
    system: Option<String>,
    tools: Vec<RegisteredTool>,
}

impl Session {
    /// Create a new session with the given dependencies.
    pub fn new(
        store: EventStore,
        client: Client,
        policy: Policy,
        tool_host: Arc<ToolHost>,
    ) -> Result<Self> {
        let id = SessionId::new();
        let event = Event::new(id, EventKind::SessionStart);
        store.append(&event)?;

        Ok(Self {
            id,
            store,
            client,
            policy,
            tool_host,
            messages: Vec::new(),
            system: None,
            tools: Vec::new(),
        })
    }

    /// Create a session without tool support (for backwards compatibility).
    pub fn new_simple(store: EventStore, client: Client, policy: Policy) -> Result<Self> {
        Self::new(store, client, policy, Arc::new(ToolHost::empty()))
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Load tools from the tool host.
    pub async fn load_tools(&mut self) -> Result<()> {
        self.tools = self.tool_host.list_tools().await;
        Ok(())
    }

    /// Build system prompt with tool instructions.
    fn build_system_prompt(&self) -> Option<String> {
        let base = self.system.clone().unwrap_or_default();

        if self.tools.is_empty() {
            if base.is_empty() {
                return None;
            }
            return Some(base);
        }

        // Build tool documentation
        let mut tool_docs = String::from("\n\n## Available Tools\n\n");
        tool_docs.push_str("You have access to the following tools. To use a tool, output:\n\n");
        tool_docs.push_str("```\n<tool_call>\n{\"name\": \"tool_name\", \"args\": {\"arg1\": \"value1\"}}\n</tool_call>\n```\n\n");
        tool_docs.push_str("Available tools:\n\n");

        for tool in &self.tools {
            tool_docs.push_str(&format!("### {}\n", tool.tool.name));
            if let Some(desc) = &tool.tool.description {
                tool_docs.push_str(&format!("{}\n", desc));
            }
            tool_docs.push_str(&format!(
                "Schema: {}\n\n",
                serde_json::to_string(&tool.tool.input_schema).unwrap_or_default()
            ));
        }

        tool_docs.push_str("After receiving tool results, continue your response. Only use tools when necessary.\n");

        Some(format!("{}{}", base, tool_docs))
    }

    /// Check if a capability is allowed by policy.
    pub fn check_capability(&self, request: &CapabilityRequest) -> Decision {
        self.policy.check(request)
    }

    /// Request a capability, returning an error if denied.
    pub fn require_capability(&self, request: &CapabilityRequest) -> Result<()> {
        match self.policy.check(request) {
            Decision::Allow => Ok(()),
            Decision::Deny { reason } => Err(Error::CapabilityDenied(reason)),
        }
    }

    /// Send a user message and get the assistant's response.
    ///
    /// This handles the full tool loop: if the model outputs tool calls,
    /// they are executed and the results fed back until the model
    /// produces a final response without tool calls.
    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        // Add user message
        let user_msg = Message::text(Role::User, user_input);
        self.messages.push(user_msg);
        self.store
            .append(&Event::message(self.id, Role::User, user_input))?;

        let system = self.build_system_prompt();

        // Tool loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > MAX_TOOL_ITERATIONS {
                return Err(Error::InvalidState(
                    "exceeded maximum tool iterations".to_string(),
                ));
            }

            // Get response from LLM (no tools param - runtime handles tools)
            let response = self.client.send(&self.messages, system.as_deref()).await?;

            let text = response.text.clone();

            // Check for tool calls in the response
            if let Some(tool_call) = self.extract_tool_call(&text) {
                // Execute tool and feed result back
                let result = self.execute_tool_call(&tool_call).await;

                // Store assistant message (with tool call)
                let assistant_msg = Message::text(Role::Assistant, text.clone());
                self.messages.push(assistant_msg);

                // Add tool result as user message
                let result_msg = format!("<tool_result>\n{result}\n</tool_result>");
                let user_msg = Message::text(Role::User, result_msg);
                self.messages.push(user_msg);
            } else {
                // No tool call - final response
                let assistant_msg = Message::text(Role::Assistant, text.clone());
                self.messages.push(assistant_msg);
                self.store
                    .append(&Event::message(self.id, Role::Assistant, &text))?;

                return Ok(text);
            }
        }
    }

    /// Extract a tool call from the response text.
    fn extract_tool_call(&self, text: &str) -> Option<ToolCall> {
        let start = text.find(TOOL_CALL_START)?;
        let end = text.find(TOOL_CALL_END)?;

        if end <= start {
            return None;
        }

        let json_str = &text[start + TOOL_CALL_START.len()..end].trim();
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;

        let name = parsed.get("name")?.as_str()?.to_string();
        let args = parsed.get("args").cloned();

        Some(ToolCall { name, args })
    }

    /// Execute a tool call and return the result as a string.
    async fn execute_tool_call(&self, call: &ToolCall) -> String {
        self.store
            .append(&Event::new(self.id, EventKind::ToolRequested))
            .ok();

        let result = self
            .tool_host
            .call_tool(&call.name, call.args.clone(), &self.policy)
            .await;

        match result {
            Ok(r) => {
                self.store
                    .append(&Event::new(self.id, EventKind::ToolSucceeded))
                    .ok();
                // Extract text from tool result
                r.content
                    .into_iter()
                    .filter_map(|c| c.as_text().map(String::from))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Err(e) => {
                self.store
                    .append(&Event::new(self.id, EventKind::ToolFailed))
                    .ok();
                format!("Error: {}", e)
            }
        }
    }

    /// End the session.
    pub fn end(self) -> Result<()> {
        self.store
            .append(&Event::new(self.id, EventKind::SessionEnd))?;
        Ok(())
    }
}

/// A parsed tool call from Claude's output.
struct ToolCall {
    name: String,
    args: Option<Value>,
}
