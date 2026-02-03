//! Core runtime for session and run management.

mod error;
pub mod llm;
mod session;
mod tools;

pub use error::{Error, Result};
pub use session::Session;
pub use tools::{RegisteredTool, ToolHost};
