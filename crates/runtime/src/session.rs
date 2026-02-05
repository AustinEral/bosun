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
    usage: Usage,
}

impl<B: Backend> Session<B> {
    /// Create a new session.
    pub fn new(store: EventStore, backend: B, policy: Policy) -> Result<Self> {
        let id = SessionId::new();
        store.append(&Event::new(id, EventKind::SessionStart))?;

        Ok(Self {
            id,
            store,
            backend,
            policy,
            messages: Vec::new(),
            usage: Usage::default(),
        })
    }

    /// Get cumulative token usage.
    pub fn usage(&self) -> Usage {
        self.usage
    }

    /// Check if a capability is allowed.
    pub fn check_capability(&self, request: &CapabilityRequest) -> Decision {
        self.policy.check(request)
    }

    /// Require a capability, error if denied.
    pub fn require_capability(&self, request: &CapabilityRequest) -> Result<()> {
        match self.policy.check(request) {
            Decision::Allow => Ok(()),
            Decision::Deny { reason } => Err(Error::CapabilityDenied(reason)),
        }
    }

    /// Chat with optional tool support.
    pub async fn chat<H: ToolHost>(
        &mut self,
        user_input: &str,
        tool_host: Option<&H>,
    ) -> Result<(String, Usage)> {
        // Add user message
        self.messages.push(Message {
            role: Role::User,
            parts: vec![Part::Text(user_input.into())],
        });
        self.log_message(StorageRole::User, user_input)?;

        let mut turn_usage = Usage::default();
        let tools = tool_host.map(|h| h.specs()).unwrap_or(&[]);

        for _ in 0..MAX_TOOL_STEPS {
            // Call model
            let response = self
                .backend
                .call(ModelRequest {
                    messages: &self.messages,
                    tools,
                })
                .await
                .map_err(|e| Error::Api(e.to_string()))?;

            turn_usage.input_tokens += response.usage.input_tokens;
            turn_usage.output_tokens += response.usage.output_tokens;

            let text = response.message.text();
            let tool_calls = response.message.tool_calls();

            self.messages.push(response.message);

            if !text.is_empty() {
                self.log_message(StorageRole::Assistant, &text)?;
            }

            // No tool calls = done
            if tool_calls.is_empty() {
                self.usage.input_tokens += turn_usage.input_tokens;
                self.usage.output_tokens += turn_usage.output_tokens;
                return Ok((text, turn_usage));
            }

            // Execute tools
            let tool_host = tool_host.ok_or_else(|| {
                Error::InvalidState("model requested tools but no tool host provided".into())
            })?;

            let results = self.execute_tools(&tool_calls, tool_host).await?;

            self.messages.push(Message {
                role: Role::User,
                parts: results,
            });
        }

        Err(Error::InvalidState("max tool steps exceeded".into()))
    }

    async fn execute_tools<H: ToolHost>(
        &self,
        calls: &[crate::model::ToolCall],
        host: &H,
    ) -> Result<Vec<Part>> {
        let mut results = Vec::with_capacity(calls.len());

        for call in calls {
            self.store.append(&Event::new(
                self.id,
                EventKind::ToolCall {
                    name: call.name.clone(),
                    input: call.input.clone(),
                },
            ))?;

            let part = match host.execute(call).await {
                Ok(output) => {
                    self.store.append(&Event::new(
                        self.id,
                        EventKind::ToolResult {
                            name: call.name.clone(),
                            output: output.clone(),
                        },
                    ))?;
                    Part::ToolResult(ToolResult::Success {
                        tool_call_id: call.id.clone(),
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
                        tool_call_id: call.id.clone(),
                        error,
                    })
                }
            };

            results.push(part);
        }

        Ok(results)
    }

    fn log_message(&self, role: StorageRole, content: &str) -> Result<()> {
        self.store.append(&Event::message(self.id, role, content))?;
        Ok(())
    }

    /// End the session.
    pub fn end(self) -> Result<()> {
        self.store
            .append(&Event::new(self.id, EventKind::SessionEnd))?;
        Ok(())
    }
}
