//! SQLite event store implementation.

/// Table name for events storage.
const EVENTS_TABLE: &str = "events";

use crate::{Error, Event, EventKind, Result, SessionId};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::Deserialize;
use std::path::Path;

/// Summary of a session for listing.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: SessionId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub message_count: u32,
}

/// Raw event row from SQLite — used for type-safe deserialization.
#[derive(Debug, Deserialize)]
struct EventRow {
    id: String,
    session_id: String,
    timestamp: String,
    data: String,
}

/// Raw session summary row from SQLite — used for type-safe deserialization.
#[derive(Debug, Deserialize)]
struct SessionRow {
    session_id: String,
    started_at: String,
    ended_at: Option<String>,
    message_count: u32,
}

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
                event.kind.name(),
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

        let rows = stmt.query_and_then([session_id.to_string()], |row| {
            serde_rusqlite::from_row::<EventRow>(row).map_err(Error::from)
        })?;

        let mut events = Vec::new();
        for row in rows {
            let row = row?;
            events.push(parse_event_row(row)?);
        }

        Ok(events)
    }

    /// List all sessions with summary info.
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                session_id,
                MIN(timestamp) as started_at,
                MAX(CASE WHEN kind = 'session_end' THEN timestamp END) as ended_at,
                SUM(CASE WHEN kind = 'message' THEN 1 ELSE 0 END) as message_count
            FROM events
            GROUP BY session_id
            ORDER BY started_at DESC
            "#,
        )?;

        let rows = stmt.query_and_then([], |row| {
            serde_rusqlite::from_row::<SessionRow>(row).map_err(Error::from)
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let row = row?;
            sessions.push(parse_session_row(row)?);
        }

        Ok(sessions)
    }

    /// Load events for a session, optionally filtering by kind.
    pub fn load_events(
        &self,
        session_id: SessionId,
        kind_filter: Option<&str>,
    ) -> Result<Vec<Event>> {
        let sql = match kind_filter {
            Some(_) => {
                "SELECT id, session_id, timestamp, data FROM events 
                 WHERE session_id = ?1 AND kind = ?2 ORDER BY timestamp"
            }
            None => {
                "SELECT id, session_id, timestamp, data FROM events 
                 WHERE session_id = ?1 ORDER BY timestamp"
            }
        };

        let mut stmt = self.conn.prepare(sql)?;

        let rows: Vec<EventRow> = if let Some(kind) = kind_filter {
            let iter = stmt.query_and_then(params![session_id.to_string(), kind], |row| {
                serde_rusqlite::from_row::<EventRow>(row).map_err(Error::from)
            })?;
            iter.collect::<Result<Vec<_>>>()?
        } else {
            let iter = stmt.query_and_then([session_id.to_string()], |row| {
                serde_rusqlite::from_row::<EventRow>(row).map_err(Error::from)
            })?;
            iter.collect::<Result<Vec<_>>>()?
        };

        let mut events = Vec::new();
        for row in rows {
            events.push(parse_event_row(row)?);
        }

        Ok(events)
    }
}

/// Parse a typed event row into an Event, with proper error reporting.
fn parse_event_row(row: EventRow) -> Result<Event> {
    let parsed_id = row.id.parse().map_err(|_| Error::Corrupted {
        table: EVENTS_TABLE,
        id: row.id.clone(),
        reason: format!("invalid UUID for event id: {}", row.id),
    })?;

    let parsed_session_id = row.session_id.parse().map_err(|_| Error::Corrupted {
        table: EVENTS_TABLE,
        id: row.id.clone(),
        reason: format!("invalid UUID for session_id: {}", row.session_id),
    })?;

    let parsed_timestamp = row.timestamp.parse().map_err(|_| Error::Corrupted {
        table: EVENTS_TABLE,
        id: row.id.clone(),
        reason: format!("invalid timestamp: {}", row.timestamp),
    })?;

    let parsed_kind: EventKind = serde_json::from_str(&row.data).map_err(|e| Error::Corrupted {
        table: EVENTS_TABLE,
        id: row.id.clone(),
        reason: format!("invalid event data: {e}"),
    })?;

    Ok(Event {
        id: parsed_id,
        session_id: SessionId(parsed_session_id),
        timestamp: parsed_timestamp,
        kind: parsed_kind,
    })
}

/// Parse a typed session summary row, with proper error reporting.
fn parse_session_row(row: SessionRow) -> Result<SessionSummary> {
    let parsed_session_id = row.session_id.parse().map_err(|_| Error::Corrupted {
        table: EVENTS_TABLE,
        id: row.session_id.clone(),
        reason: format!("invalid UUID for session_id: {}", row.session_id),
    })?;

    let parsed_started_at = row.started_at.parse().map_err(|_| Error::Corrupted {
        table: EVENTS_TABLE,
        id: row.session_id.clone(),
        reason: format!("invalid started_at timestamp: {}", row.started_at),
    })?;

    let parsed_ended_at = match row.ended_at {
        Some(ts) => Some(ts.parse().map_err(|_| Error::Corrupted {
            table: EVENTS_TABLE,
            id: row.session_id.clone(),
            reason: format!("invalid ended_at timestamp: {ts}"),
        })?),
        None => None,
    };

    Ok(SessionSummary {
        id: SessionId(parsed_session_id),
        started_at: parsed_started_at,
        ended_at: parsed_ended_at,
        message_count: row.message_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Role;

    #[test]
    fn test_append_and_load_events() {
        let store = EventStore::in_memory().unwrap();
        let session_id = SessionId::new();

        // Append a session start event
        let start_event = Event::new(session_id, EventKind::SessionStart);
        store.append(&start_event).unwrap();

        // Append a message
        let msg_event = Event::message(session_id, Role::User, "Hello, Bosun!");
        store.append(&msg_event).unwrap();

        // Load and verify
        let events = store.load_session(session_id).unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].kind, EventKind::SessionStart));
        assert!(matches!(
            events[1].kind,
            EventKind::Message {
                role: Role::User,
                ..
            }
        ));
    }

    #[test]
    fn test_list_sessions() {
        let store = EventStore::in_memory().unwrap();

        // Create two sessions
        let session1 = SessionId::new();
        let session2 = SessionId::new();

        store
            .append(&Event::new(session1, EventKind::SessionStart))
            .unwrap();
        store
            .append(&Event::message(session1, Role::User, "First"))
            .unwrap();
        store
            .append(&Event::message(session1, Role::Assistant, "Reply"))
            .unwrap();

        store
            .append(&Event::new(session2, EventKind::SessionStart))
            .unwrap();
        store
            .append(&Event::message(session2, Role::User, "Second"))
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        // Check message counts (session1 has 2 messages, session2 has 1)
        let s1 = sessions.iter().find(|s| s.id == session1).unwrap();
        let s2 = sessions.iter().find(|s| s.id == session2).unwrap();
        assert_eq!(s1.message_count, 2);
        assert_eq!(s2.message_count, 1);
    }

    #[test]
    fn test_load_events_with_filter() {
        let store = EventStore::in_memory().unwrap();
        let session_id = SessionId::new();

        store
            .append(&Event::new(session_id, EventKind::SessionStart))
            .unwrap();
        store
            .append(&Event::message(session_id, Role::User, "Hello"))
            .unwrap();
        store
            .append(&Event::message(session_id, Role::Assistant, "Hi"))
            .unwrap();
        store
            .append(&Event::new(session_id, EventKind::SessionEnd))
            .unwrap();

        // Filter by message kind
        let messages = store.load_events(session_id, Some("message")).unwrap();
        assert_eq!(messages.len(), 2);

        // Filter by session_start
        let starts = store
            .load_events(session_id, Some("session_start"))
            .unwrap();
        assert_eq!(starts.len(), 1);

        // No filter
        let all = store.load_events(session_id, None).unwrap();
        assert_eq!(all.len(), 4);
    }
}
