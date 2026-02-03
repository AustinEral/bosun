//! Session management.

use std::sync::Arc;

use crate::llm::{Client, ContentBlock, LlmResponse, Message, ToolDefinition};
use crate::tools::ToolHost;
use crate::{Error, Result};
use policy::{CapabilityRequest, Decision, Policy};
use serde_json::Value;
use storage::{Event, EventKind, EventStore, Role, SessionId};

/// Maximum tool iterations per run to prevent infinite loops.
const MAX_TOOL_ITERATIONS: usize = 10;

/// A conversation session with tool support.
pub struct Session {
    pub id: SessionId,
    store: EventStore,
    client: Client,
    policy: Policy,
    tool_host: Arc<ToolHost>,
    messages: Vec<Message>,
    system: Option<String>,
    tools: Vec<ToolDefinition>,
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
        let registered = self.tool_host.list_tools().await;
        self.tools = registered.iter().map(|r| ToolDefinition::from(&r.tool)).collect();
        Ok(())
    }

    /// Add a custom tool definition.
    pub fn add_tool(&mut self, tool: ToolDefinition) {
        self.tools.push(tool);
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
    /// This handles the full tool loop: if the model requests tools,
    /// they are executed and the results fed back until the model
    /// produces a final text response.
    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        // Add user message
        let user_msg = Message::text(Role::User, user_input);
        self.messages.push(user_msg);
        self.store
            .append(&Event::message(self.id, Role::User, user_input))?;

        // Tool loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > MAX_TOOL_ITERATIONS {
                return Err(Error::InvalidState(
                    "exceeded maximum tool iterations".to_string(),
                ));
            }

            // Get response from LLM
            let tools = if self.tools.is_empty() {
                None
            } else {
                Some(self.tools.as_slice())
            };

            let response = self
                .client
                .send(&self.messages, self.system.as_deref(), tools)
                .await?;

            // Log token usage
            self.log_usage(&response);

            if response.has_tool_use() {
                // Handle tool calls
                self.handle_tool_response(response).await?;
            } else {
                // Final response - extract text and return
                let text = response.text();
                
                // Store assistant message
                let assistant_msg = Message::text(Role::Assistant, &text);
                self.messages.push(assistant_msg);
                self.store
                    .append(&Event::message(self.id, Role::Assistant, &text))?;

                return Ok(text);
            }
        }
    }

    /// Handle a response that includes tool use requests.
    async fn handle_tool_response(&mut self, response: LlmResponse) -> Result<()> {
        // Store assistant message with tool use
        let assistant_msg = Message::blocks(Role::Assistant, response.content.clone());
        self.messages.push(assistant_msg);

        // Execute each tool and collect results
        let mut results = Vec::new();
        for (id, name, input) in response.tool_uses() {
            self.store.append(&Event::new(self.id, EventKind::ToolRequested))?;

            let result = self.execute_tool(name, input.clone()).await;
            
            let block = match result {
                Ok(output) => {
                    self.store.append(&Event::new(self.id, EventKind::ToolSucceeded))?;
                    ContentBlock::tool_result(id, output)
                }
                Err(e) => {
                    self.store.append(&Event::new(self.id, EventKind::ToolFailed))?;
                    ContentBlock::tool_error(id, e.to_string())
                }
            };
            results.push(block);
        }

        // Add tool results as user message
        let user_msg = Message::blocks(Role::User, results);
        self.messages.push(user_msg);

        Ok(())
    }

    /// Execute a single tool call.
    async fn execute_tool(&self, name: &str, input: Value) -> Result<String> {
        let params = if input.is_null() || input == Value::Object(Default::default()) {
            None
        } else {
            Some(input)
        };

        let result = self.tool_host.call_tool(name, params, &self.policy).await?;

        // Convert result to string
        let output = result
            .content
            .into_iter()
            .filter_map(|c| c.as_text().map(String::from))
            .collect::<Vec<_>>()
            .join("\n");

        if result.is_error {
            Err(Error::Tool(output))
        } else {
            Ok(output)
        }
    }

    fn log_usage(&self, _response: &LlmResponse) {
        // TODO: Log to event store once we have proper usage events
        // For now, usage is captured in LlmResponse but not persisted
    }

    /// End the session.
    pub fn end(self) -> Result<()> {
        self.store
            .append(&Event::new(self.id, EventKind::SessionEnd))?;
        Ok(())
    }
}
