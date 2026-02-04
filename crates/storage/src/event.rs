//! Event types for the event log.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// A unique identifier for a session.
///
/// Session IDs are UUIDs that uniquely identify a conversation session.
/// They can be displayed as strings and parsed back from strings.
///
/// This type intentionally does not implement `Default` because there is
/// no sensible default value for a unique identifier.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use storage::SessionId;
///
/// // Create a new session ID
/// let id = SessionId::new();
///
/// // Convert to string and parse back
/// let id_str = id.to_string();
/// let parsed: SessionId = id_str.parse().unwrap();
/// assert_eq!(id, parsed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

// Clippy wants Default for types with new(), but SessionId generates
// a unique random value each time - there is no sensible default.
#[allow(clippy::new_without_default)]
impl SessionId {
    /// Creates a new unique session identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
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

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::System => write!(f, "system"),
        }
    }
}

/// An error returned when parsing a [`Role`] from a string fails.
///
/// This follows the pattern of [`std::str::ParseBoolError`] and other
/// standard library parse error types.
///
/// # Example
///
/// ```
/// use std::str::FromStr;
/// use storage::Role;
///
/// let result: Result<Role, _> = "invalid".parse();
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRoleError {
    input: String,
}

impl ParseRoleError {
    fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
        }
    }

    /// Returns the input that failed to parse.
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ParseRoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown role: '{}' (expected 'user', 'assistant', or 'system')",
            self.input
        )
    }
}

impl std::error::Error for ParseRoleError {}

impl FromStr for Role {
    type Err = ParseRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            "system" => Ok(Role::System),
            _ => Err(ParseRoleError::new(s)),
        }
    }
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

impl EventKind {
    /// Returns the canonical name of this event kind.
    ///
    /// This matches the serialized `kind` field and is used for storage indexing.
    ///
    /// # Example
    ///
    /// ```
    /// use storage::EventKind;
    ///
    /// assert_eq!(EventKind::SessionStart.name(), "session_start");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Self::Message { .. } => "message",
            Self::ToolCall { .. } => "tool_call",
            Self::ToolResult { .. } => "tool_result",
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_name_matches_serde() {
        // Verify that name() returns values consistent with serde serialization
        assert_eq!(EventKind::SessionStart.name(), "session_start");
        assert_eq!(EventKind::SessionEnd.name(), "session_end");
        assert_eq!(
            EventKind::Message {
                role: Role::User,
                content: "test".into()
            }
            .name(),
            "message"
        );
        assert_eq!(
            EventKind::ToolCall {
                name: "test".into(),
                input: serde_json::Value::Null
            }
            .name(),
            "tool_call"
        );
        assert_eq!(
            EventKind::ToolResult {
                name: "test".into(),
                output: serde_json::Value::Null
            }
            .name(),
            "tool_result"
        );
    }

    #[test]
    fn session_id_roundtrip() {
        let id = SessionId::new();
        let s = id.to_string();
        let parsed: SessionId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn session_id_parse_invalid() {
        let result: Result<SessionId, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn role_parse_valid() {
        assert_eq!("user".parse::<Role>().unwrap(), Role::User);
        assert_eq!("assistant".parse::<Role>().unwrap(), Role::Assistant);
        assert_eq!("system".parse::<Role>().unwrap(), Role::System);
        // Case insensitive
        assert_eq!("USER".parse::<Role>().unwrap(), Role::User);
        assert_eq!("Assistant".parse::<Role>().unwrap(), Role::Assistant);
    }

    #[test]
    fn role_parse_invalid() {
        let result: Result<Role, ParseRoleError> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.input(), "invalid");
        assert!(err.to_string().contains("unknown role"));
    }

    #[test]
    fn role_roundtrip() {
        for role in [Role::User, Role::Assistant, Role::System] {
            let s = role.to_string();
            let parsed: Role = s.parse().unwrap();
            assert_eq!(role, parsed);
        }
    }
}
