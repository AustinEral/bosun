//! Empty tool host implementation.

use crate::model::{ToolCall, ToolSpec};
use crate::tools::{ToolError, ToolHost};
use serde_json::Value;

/// A no-op tool host with no tools.
///
/// Useful for testing or when tools are not needed.
#[derive(Debug, Default)]
pub struct EmptyToolHost;

impl ToolHost for EmptyToolHost {
    fn specs(&self) -> &[ToolSpec] {
        &[]
    }

    async fn execute(&self, call: &ToolCall) -> Result<Value, ToolError> {
        Err(ToolError::NotFound(call.name.clone()))
    }
}
