//! LLM backend abstraction.
//!
//! Provides a trait for LLM backends, allowing Bosun to support multiple
//! providers (Anthropic API, OpenAI, etc.) through a unified interface.

mod anthropic;

pub use anthropic::{AnthropicAuth, AnthropicBackend};

use crate::Result;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use storage::Role;

/// A message in the conversation.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
}

/// Request to send to an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatRequest<'a> {
    pub messages: &'a [Message],
    pub system: Option<&'a str>,
}

/// Token usage information from an LLM response.
///
/// Tracks input (prompt) and output (completion) token counts.
/// Implements arithmetic traits for easy accumulation across turns.
///
/// # Example
///
/// ```
/// use runtime::Usage;
///
/// let turn1 = Usage { input_tokens: 100, output_tokens: 50 };
/// let turn2 = Usage { input_tokens: 150, output_tokens: 75 };
///
/// // Combine with + operator
/// let combined = turn1 + turn2;
/// assert_eq!(combined.input_tokens, 250);
/// assert_eq!(combined.output_tokens, 125);
///
/// // Accumulate with +=
/// let mut total = Usage::default();
/// total += turn1;
/// total += turn2;
/// assert_eq!(total, combined);
///
/// // Get total token count
/// assert_eq!(combined.total(), 375);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens consumed by the input (prompt).
    pub input_tokens: u32,
    /// Tokens generated in the output (completion).
    pub output_tokens: u32,
}

impl Usage {
    /// Returns the total token count (input + output).
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

impl Add for Usage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
        }
    }
}

impl AddAssign for Usage {
    fn add_assign(&mut self, other: Self) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
    }
}

impl Sum for Usage {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, x| acc + x)
    }
}

/// Response from an LLM backend.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The generated content.
    pub content: String,
    /// Token usage statistics.
    pub usage: Usage,
}

/// Trait for LLM backends.
///
/// Implementations handle the specifics of communicating with different
/// LLM providers (API calls, etc.).
pub trait LlmBackend: Send + Sync {
    /// Send a chat request and get a response.
    fn chat(&self, request: ChatRequest<'_>) -> impl Future<Output = Result<ChatResponse>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_add() {
        let a = Usage { input_tokens: 100, output_tokens: 50 };
        let b = Usage { input_tokens: 200, output_tokens: 75 };
        let combined = a + b;
        assert_eq!(combined.input_tokens, 300);
        assert_eq!(combined.output_tokens, 125);
    }

    #[test]
    fn usage_add_assign() {
        let mut total = Usage::default();
        total += Usage { input_tokens: 100, output_tokens: 50 };
        total += Usage { input_tokens: 200, output_tokens: 75 };
        assert_eq!(total.input_tokens, 300);
        assert_eq!(total.output_tokens, 125);
    }

    #[test]
    fn usage_sum() {
        let usages = vec![
            Usage { input_tokens: 100, output_tokens: 50 },
            Usage { input_tokens: 200, output_tokens: 75 },
            Usage { input_tokens: 50, output_tokens: 25 },
        ];
        let total: Usage = usages.into_iter().sum();
        assert_eq!(total.input_tokens, 350);
        assert_eq!(total.output_tokens, 150);
    }

    #[test]
    fn usage_total() {
        let usage = Usage { input_tokens: 100, output_tokens: 50 };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn usage_default_is_zero() {
        let usage = Usage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total(), 0);
    }

    #[test]
    fn usage_equality() {
        let a = Usage { input_tokens: 100, output_tokens: 50 };
        let b = Usage { input_tokens: 100, output_tokens: 50 };
        let c = Usage { input_tokens: 100, output_tokens: 51 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
