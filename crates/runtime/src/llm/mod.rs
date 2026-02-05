//! LLM abstraction layer.
//!
//! Provider-agnostic types and traits for LLM interactions.

mod types;

pub use types::{
    FinishReason, Message, Part, Role, ToolCall, ToolChoice, ToolResult, ToolSpec, Usage,
};
