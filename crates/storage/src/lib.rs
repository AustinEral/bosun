//! SQLite-backed event storage for Bosun sessions.
//!
//! This crate provides persistent storage for Bosun's event log — the complete
//! audit trail of everything that happens during agent sessions. Every message,
//! tool call, and session lifecycle event is captured and queryable.
//!
//! # Overview
//!
//! The storage layer serves two purposes:
//!
//! 1. **Audit Trail** — Complete history of all agent interactions, enabling
//!    "why did it do that?" debugging and compliance requirements.
//!
//! 2. **Session Management** — Track session state, list past sessions, and
//!    enable session resumption (future).
//!
//! # Core Concepts
//!
//! ## EventStore
//!
//! The [`EventStore`] is the primary interface for persistence. It wraps a SQLite
//! database and provides methods to append events and query session history.
//!
//! ## Event
//!
//! An [`Event`] represents something that happened during a session. Each event has:
//! - A unique ID
//! - A session ID linking it to a conversation
//! - A timestamp
//! - A kind describing what happened ([`EventKind`])
//!
//! ## EventKind
//!
//! The [`EventKind`] enum captures the different types of events:
//! - `SessionStart` / `SessionEnd` — Session lifecycle
//! - `Message` — User or assistant messages
//! - `ToolCall` / `ToolResult` — Tool invocations and their results
//!
//! ## SessionId
//!
//! A [`SessionId`] is a UUID that uniquely identifies a conversation session.
//! It can be displayed as a string and parsed back, enabling CLI commands like
//! `bosun logs --session abc123`.
//!
//! # Example
//!
//! ```no_run
//! use storage::{EventStore, Event, EventKind, Role, SessionId};
//!
//! // Open or create the event store
//! let store = EventStore::open("events.db")?;
//!
//! // Start a new session
//! let session_id = SessionId::new();
//! store.append(&Event::new(session_id, EventKind::SessionStart))?;
//!
//! // Log a user message
//! store.append(&Event::message(session_id, Role::User, "Hello, Bosun!"))?;
//!
//! // Log an assistant response
//! store.append(&Event::message(session_id, Role::Assistant, "Hello! How can I help?"))?;
//!
//! // Query session history
//! let events = store.load_session(session_id)?;
//! for event in events {
//!     println!("{}: {:?}", event.timestamp, event.kind);
//! }
//!
//! // List all sessions
//! let sessions = store.list_sessions()?;
//! for summary in sessions {
//!     println!("{}: {} messages", summary.id, summary.message_count);
//! }
//! # Ok::<(), storage::Error>(())
//! ```
//!
//! # Re-exports
//!
//! This crate re-exports all public types at the crate root for convenience:
//!
//! - [`EventStore`], [`SessionSummary`] — Storage interface
//! - [`Event`], [`EventKind`] — Event types
//! - [`SessionId`], [`Role`] — Domain types
//! - [`Error`], [`Result`] — Error handling

mod error;
mod event;
mod store;

pub use error::{Error, Result};
pub use event::{Event, EventKind, Role, SessionId};
pub use store::{EventStore, SessionSummary};
