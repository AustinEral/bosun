//! Session management.

use crate::llm::{Client, Message};
use crate::Result;
use storage::{Event, EventKind, EventStore, Role, SessionId};

/// A conversation session.
pub struct Session {
    pub id: SessionId,
    store: EventStore,
    client: Client,
    messages: Vec<Message>,
    system: Option<String>,
}

impl Session {
    /// Create a new session with the given store and client.
    pub fn new(store: EventStore, client: Client) -> Result<Self> {
        let id = SessionId::new();
        let event = Event::new(id, EventKind::SessionStart);
        store.append(&event)?;

        Ok(Self {
            id,
            store,
            client,
            messages: Vec::new(),
            system: None,
        })
    }

    /// Set the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Send a user message and get the assistant's response.
    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        // Log and store user message
        let user_msg = Message {
            role: Role::User,
            content: user_input.to_string(),
        };
        self.messages.push(user_msg);
        self.store
            .append(&Event::message(self.id, Role::User, user_input))?;

        // Get response from LLM
        let response = self
            .client
            .send(&self.messages, self.system.as_deref())
            .await?;

        // Log and store assistant message
        let assistant_msg = Message {
            role: Role::Assistant,
            content: response.clone(),
        };
        self.messages.push(assistant_msg);
        self.store
            .append(&Event::message(self.id, Role::Assistant, &response))?;

        Ok(response)
    }

    /// End the session.
    pub fn end(self) -> Result<()> {
        self.store
            .append(&Event::new(self.id, EventKind::SessionEnd))?;
        Ok(())
    }
}
