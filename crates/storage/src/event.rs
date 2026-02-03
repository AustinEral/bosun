//! Event types for the event log.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The role of a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

/// The kind of event that occurred.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    /// A message was added to the conversation.
    Message { role: Role, content: String },
    /// A tool was invoked.
    ToolCall {
        name: String,
        input: serde_json::Value,
    },
    /// A tool returned a result.
    ToolResult {
        name: String,
        output: serde_json::Value,
    },
    /// Session started.
    SessionStart,
    /// Session ended.
    SessionEnd,
}

/// An event in the session log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub session_id: SessionId,
    pub timestamp: DateTime<Utc>,
    pub kind: EventKind,
}

impl Event {
    pub fn new(session_id: SessionId, kind: EventKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            timestamp: Utc::now(),
            kind,
        }
    }

    pub fn message(session_id: SessionId, role: Role, content: impl Into<String>) -> Self {
        Self::new(
            session_id,
            EventKind::Message {
                role,
                content: content.into(),
            },
        )
    }
}
