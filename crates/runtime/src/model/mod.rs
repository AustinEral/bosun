//! Model protocol types and backend trait.

pub mod backend;
pub mod errors;
pub mod types;

pub use backend::{AnthropicAuth, AnthropicBackend, AnthropicBackendBuilder};
pub use errors::ModelError;
pub use types::{Backend, Message, ModelRequest, ModelResponse, Part, Role, Usage};
