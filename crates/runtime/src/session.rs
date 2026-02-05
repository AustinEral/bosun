//! Session management.

use crate::model::{Backend, Message, ModelRequest, Part, Role, ToolResult, Usage};
use crate::tools::ToolHost;
use crate::{Error, Result};
use policy::{CapabilityRequest, Decision, Policy};
use serde_json::json;
use storage::{Event, EventKind, EventStore, Role as StorageRole, SessionId};

const MAX_TOOL_STEPS: usize = 8;

/// A conversation session.
pub struct Session<B: Backend> {
    pub id: SessionId,
    store: EventStore,
    backend: B,
    policy: Policy,
    messages: Vec<Message>,
    system: Option<String>,
    usage: Usage,
}

impl<B: Backend> Session<B> {
    /// Create a new session with the given store, backend, and policy.
    pub fn new(store: EventStore, backend: B, policy: Policy) -> Result<Self> {
        let id = SessionId::new();
        let event = Event::new(id, EventKind::SessionStart);
        store.append(&event)?;

        Ok(Self {
            id,
            store,
            backend,
            policy,
            messages: Vec::new(),
            system: None,
            usage: Usage::default(),
        })
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Get cumulative token usage for this session.
    pub fn usage(&self) -> Usage {
        self.usage
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

    /// Send a user message and get the assistant's response (no tools).
    pub async fn chat(&mut self, user_input: &str) -> Result<(String, Usage)> {
        self.messages.push(Message::user(user_input));
        self.store
            .append(&Event::message(self.id, StorageRole::User, user_input))?;

        let request = ModelRequest {
            messages: &self.messages,
            tools: &[],
        };
        let response = self
            .backend
            .call(request)
            .await
            .map_err(|e| Error::Api(e.to_string()))?;

        let text = response.message.text();
        self.messages.push(response.message);
        self.store
            .append(&Event::message(self.id, StorageRole::Assistant, &text))?;

        self.usage.input_tokens += response.usage.input_tokens;
        self.usage.output_tokens += response.usage.output_tokens;

        Ok((text, response.usage))
    }

    /// Send a user message with tool support.
    pub async fn chat_with_tools<H: ToolHost>(
        &mut self,
        user_input: &str,
        tool_host: &H,
    ) -> Result<(String, Usage)> {
        self.messages.push(Message::user(user_input));
        self.store
            .append(&Event::message(self.id, StorageRole::User, user_input))?;

        let mut turn_usage = Usage::default();

        for _ in 0..MAX_TOOL_STEPS {
            let request = ModelRequest {
                messages: &self.messages,
                tools: tool_host.specs(),
            };

            let response = self
                .backend
                .call(request)
                .await
                .map_err(|e| Error::Api(e.to_string()))?;
            turn_usage.input_tokens += response.usage.input_tokens;
            turn_usage.output_tokens += response.usage.output_tokens;

            let assistant_text = response.message.text();
            let tool_calls = response.message.tool_calls();

            self.messages.push(response.message);

            if !assistant_text.is_empty() {
                self.store.append(&Event::message(
                    self.id,
                    StorageRole::Assistant,
                    &assistant_text,
                ))?;
            }

            // No tool calls = done
            if tool_calls.is_empty() {
                self.usage.input_tokens += turn_usage.input_tokens;
                self.usage.output_tokens += turn_usage.output_tokens;
                return Ok((assistant_text, turn_usage));
            }

            // Execute tools and collect results
            let mut tool_result_parts = Vec::with_capacity(tool_calls.len());

            for call in tool_calls {
                // Log tool call
                self.store.append(&Event::new(
                    self.id,
                    EventKind::ToolCall {
                        name: call.name.clone(),
                        input: call.input.clone(),
                    },
                ))?;

                let result_part = match tool_host.execute(&call).await {
                    Ok(output) => {
                        self.store.append(&Event::new(
                            self.id,
                            EventKind::ToolResult {
                                name: call.name.clone(),
                                output: output.clone(),
                            },
                        ))?;
                        Part::ToolResult(ToolResult::Success {
                            tool_call_id: call.id,
                            output,
                        })
                    }
                    Err(error) => {
                        self.store.append(&Event::new(
                            self.id,
                            EventKind::ToolResult {
                                name: call.name.clone(),
                                output: json!({ "error": error.to_string() }),
                            },
                        ))?;
                        Part::ToolResult(ToolResult::Failure {
                            tool_call_id: call.id,
                            error,
                        })
                    }
                };

                tool_result_parts.push(result_part);
            }

            // Add tool results as user message
            self.messages.push(Message {
                role: Role::User,
                parts: tool_result_parts,
            });
        }

        Err(Error::InvalidState("max tool steps exceeded".into()))
    }

    /// End the session.
    pub fn end(self) -> Result<()> {
        self.store
            .append(&Event::new(self.id, EventKind::SessionEnd))?;
        Ok(())
    }
}
