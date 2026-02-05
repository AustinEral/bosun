//! LLM protocol types and backend trait.

pub mod errors;
pub mod types;

pub use errors::ModelError;
pub use types::{
    Backend, Message, ModelRequest, ModelResponse, Part, Role, ToolCall, ToolResult, ToolSpec,
};
