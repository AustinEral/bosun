//! SQLite event store implementation.

use crate::{Event, Result, SessionId};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::path::Path;

/// Summary of a session for listing.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: SessionId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub message_count: u32,
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

    /// List all sessions with summary info.
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                session_id,
                MIN(timestamp) as started_at,
                MAX(CASE WHEN kind = session_end THEN timestamp END) as ended_at,
                SUM(CASE WHEN kind = message THEN 1 ELSE 0 END) as message_count
            FROM events
            GROUP BY session_id
            ORDER BY started_at DESC
            "#,
        )?;

        let sessions = stmt
            .query_map([], |row| {
                let session_id: String = row.get(0)?;
                let started_at: String = row.get(1)?;
                let ended_at: Option<String> = row.get(2)?;
                let message_count: u32 = row.get(3)?;
                Ok((session_id, started_at, ended_at, message_count))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(session_id, started_at, ended_at, message_count)| {
                Some(SessionSummary {
                    id: SessionId(session_id.parse().ok()?),
                    started_at: started_at.parse().ok()?,
                    ended_at: ended_at.and_then(|s| s.parse().ok()),
                    message_count,
                })
            })
            .collect();

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

        let rows: Vec<(String, String, String, String)> = if let Some(kind) = kind_filter {
            stmt.query_map(params![session_id.to_string(), kind], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map([session_id.to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .filter_map(|r| r.ok())
            .collect()
        };

        let events = rows
            .into_iter()
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
