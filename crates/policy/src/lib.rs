//! Capability-based policy system.
//!
//! Core principle: **All side effects require an explicit capability.**

mod capability;
mod error;
mod policy;

pub use capability::{CapabilityKind, CapabilityRequest};
pub use error::{Error, Result};
pub use policy::{Decision, Policy};
