//! SQLite storage backend.

mod error;
mod event;
mod store;

pub use error::{Error, Result};
pub use event::{Event, EventKind, ParseRoleError, Role, SessionId};
pub use store::{EventStore, SessionSummary};
