//! Core runtime for session and run management.

mod error;
pub mod llm;
mod session;

pub use error::{Error, Result};
pub use session::Session;
