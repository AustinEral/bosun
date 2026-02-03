//! Capability-based policy system.
//!
//! Core principle: **All side effects require an explicit capability.**

mod capability;
mod error;

pub use capability::Capability;
pub use error::{Error, Result};
