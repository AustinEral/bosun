//! Session management.

use crate::backend::{ChatRequest, LlmBackend, Message};
use crate::{Error, Result};
use policy::{CapabilityRequest, Decision, Policy};
use storage::{Event, EventKind, EventStore, Role, SessionId};

/// A conversation session.
pub struct Session<B: LlmBackend> {
    pub id: SessionId,
    store: EventStore,
    backend: B,
    policy: Policy,
    messages: Vec<Message>,
    system: Option<String>,
}

impl<B: LlmBackend> Session<B> {
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
        })
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
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
    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        let user_msg = Message::user(user_input);
        self.messages.push(user_msg);
        self.store
            .append(&Event::message(self.id, Role::User, user_input))?;

        let request = ChatRequest {
            messages: &self.messages,
            system: self.system.as_deref(),
        };
        let response = self.backend.chat(request).await?;

        let assistant_msg = Message::assistant(&response.content);
        self.messages.push(assistant_msg);
        self.store
            .append(&Event::message(self.id, Role::Assistant, &response.content))?;

        Ok(response.content)
    }

    /// End the session.
    pub fn end(self) -> Result<()> {
        self.store
            .append(&Event::new(self.id, EventKind::SessionEnd))?;
        Ok(())
    }
}
