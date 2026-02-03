//! SQLite event store implementation.

use crate::{Event, EventKind, Result, SessionId};
use rusqlite::{params, Connection};
use std::path::Path;

/// SQLite-backed event store.
pub struct EventStore {
    conn: Connection,
}

impl EventStore {
    /// Open or create an event store at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Create an in-memory event store (useful for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                kind TEXT NOT NULL,
                data TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_session 
                ON events(session_id, timestamp);
            "#,
        )?;
        Ok(())
    }

    /// Append an event to the store.
    pub fn append(&self, event: &Event) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (id, session_id, timestamp, kind, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.id.to_string(),
                event.session_id.to_string(),
                event.timestamp.to_rfc3339(),
                event_kind_name(&event.kind),
                serde_json::to_string(&event.kind)?,
            ],
        )?;
        Ok(())
    }

    /// Load all events for a session, ordered by timestamp.
    pub fn load_session(&self, session_id: SessionId) -> Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, timestamp, data FROM events 
             WHERE session_id = ?1 ORDER BY timestamp",
        )?;

        let events = stmt
            .query_map([session_id.to_string()], |row| {
                let id: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let timestamp: String = row.get(2)?;
                let data: String = row.get(3)?;
                Ok((id, session_id, timestamp, data))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, session_id, timestamp, data)| {
                Some(Event {
                    id: id.parse().ok()?,
                    session_id: SessionId(session_id.parse().ok()?),
                    timestamp: timestamp.parse().ok()?,
                    kind: serde_json::from_str(&data).ok()?,
                })
            })
            .collect();

        Ok(events)
    }
}

fn event_kind_name(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Message { .. } => "message",
        EventKind::ToolCall { .. } => "tool_call",
        EventKind::ToolResult { .. } => "tool_result",
        EventKind::SessionStart => "session_start",
        EventKind::SessionEnd => "session_end",
    }
}
