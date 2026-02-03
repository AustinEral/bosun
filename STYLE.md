# Bosun — Rust Style Guide

A practical style guide for Bosun development. Combines official Rust guidelines with lessons from excellent projects.

---

## Quick Reference

```bash
# Before committing
cargo fmt          # Format code
cargo clippy       # Lint
cargo test         # Run tests
cargo doc --open   # Check docs build
```

---

## Core Principles

### Human Readability First

Code is read far more often than it's written. Optimize for the next person who needs to understand this code.

```rust
// Good: clear intent
pub fn token_cost(self) -> f64 {
    match self {
        Model::Sonnet => 0.003,
        Model::Opus => 0.015,
    }
}

// Bad: cryptic
pub fn tc(self) -> f64 {
    if self == Model::Sonnet { 0.003 } else { 0.015 }
}
```

### YAGNI (You Aren't Gonna Need It)

Build only what's required now. Don't add functionality based on speculation.

- ✅ Implement what the spec requires
- ❌ Don't add features "just in case"
- ✅ Minimal stubs marked clearly as future work
- ❌ Don't build full APIs before they're needed

### All Side Effects Require Explicit Capability

This is Bosun's core principle. It applies to code design too — make side effects visible and intentional.

---

## 1. Formatting

Follow `rustfmt` defaults. Don't fight the formatter.

### Basics

- **Indentation:** 4 spaces (no tabs)
- **Line width:** 100 characters max
- **Trailing commas:** Always in multi-line constructs
- **Blank lines:** One between items, zero or one between statements

```rust
// Good: trailing commas, block indent
let config = Config {
    name: "bosun".to_string(),
    version: Version::new(0, 1, 0),
    debug: true,
};
```

### Import Organization

All `use` statements at the top, never inline. Group with blank lines:

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// 3. Crate root items
use crate::config::Config;
use crate::error::Result;

// 4. Parent/sibling modules
use super::storage;
```

**When to keep module prefix:**
- Adds clarity (`event::emit` vs just `emit`)
- Prevents name conflicts
- Groups related functions conceptually

---

## 2. Naming

Follow [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html).

| Item | Convention | Example |
|------|------------|---------|
| Crates | `snake_case` | `bosun_runtime` |
| Modules | `snake_case` | `event_store` |
| Types | `UpperCamelCase` | `SessionManager` |
| Functions, methods | `snake_case` | `run_session` |
| Local variables | `snake_case` | `event_count` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES` |
| Type parameters | `UpperCamelCase`, short | `T`, `E`, `R` |
| Lifetimes | `lowercase`, short | `'a`, `'ctx` |

### Conversions

```rust
// as_ — cheap, borrowed view
fn as_bytes(&self) -> &[u8]

// to_ — expensive conversion, new allocation
fn to_string(&self) -> String

// into_ — consuming conversion
fn into_inner(self) -> T
```

### Getters (no `get_` prefix)

```rust
fn name(&self) -> &str
fn is_empty(&self) -> bool
fn has_children(&self) -> bool
```

### Iterators

```rust
fn iter(&self) -> Iter<'_, T>
fn iter_mut(&mut self) -> IterMut<'_, T>
fn into_iter(self) -> IntoIter<T>
```

---

## 3. Types & Structures

### Domain Modeling with Enums

Use enums to represent domain concepts with finite valid values.

```rust
// Good: type system enforces valid values
pub enum SessionState {
    Active,
    Paused,
    Waiting { for_event: EventKind },
    Ended { reason: EndReason },
}

// Bad: stringly-typed
pub struct Session {
    state: String,  // "active" | "paused" ???
}

// Bad: boolean blindness
pub struct Session {
    is_paused: bool,  // What about other states?
}
```

### Newtypes for Clarity

```rust
// Good: distinct types prevent mixing up IDs
struct SessionId(Ulid);
struct RunId(Ulid);

fn start_run(session: SessionId, run: RunId);  // Can't swap accidentally

// Bad: easy to confuse
fn start_run(session: Ulid, run: Ulid);
```

### Builder Pattern for Complex Construction

```rust
let config = ConfigBuilder::new()
    .model("claude-sonnet-4-20250514")
    .max_tokens(8000)
    .timeout(Duration::from_secs(30))
    .build()?;
```

---

## 4. Strategy Pattern

When behavior varies based on a type, avoid scattering conditionals. Use **enum dispatch** or **trait-based strategies**.

### Enum Dispatch

Use when you have a **closed set** of variants with minimal per-variant state.

```rust
#[derive(Clone, Copy)]
pub enum EventKind {
    RunStarted,
    RunEnded,
    ToolInvoked,
}

impl EventKind {
    pub fn is_terminal(self) -> bool {
        match self {
            EventKind::RunStarted => false,
            EventKind::RunEnded => true,
            EventKind::ToolInvoked => false,
        }
    }
}

// Usage is clean
if event.kind.is_terminal() { /* ... */ }
```

**Advantages:** Zero-cost, Copy-able, exhaustiveness checking, better optimization.

### Trait-Based Strategy

Use when variants need **different initialization parameters** or **encapsulated state**.

```rust
pub trait LlmAdapter {
    fn complete(&self, request: &Request) -> Result<Response>;
    fn name(&self) -> &str;
}

pub struct AnthropicAdapter {
    api_key: String,
    model: String,
}

impl LlmAdapter for AnthropicAdapter {
    fn complete(&self, request: &Request) -> Result<Response> {
        // Uses self.api_key, self.model
        todo!()
    }
    fn name(&self) -> &str { "anthropic" }
}

pub struct OpenAiAdapter {
    api_key: String,
    org_id: Option<String>,  // Different config than Anthropic
}

impl LlmAdapter for OpenAiAdapter {
    fn complete(&self, request: &Request) -> Result<Response> {
        // Uses self.api_key, self.org_id
        todo!()
    }
    fn name(&self) -> &str { "openai" }
}

// Generic over adapter type (static dispatch)
pub struct Runtime<A: LlmAdapter> {
    adapter: A,
}
```

### Choosing Between Enum and Trait

| Factor | Enum Dispatch | Trait Strategy |
|--------|--------------|----------------|
| Variants | Closed set | Closed or extensible |
| State | Minimal, uniform | Varies by strategy |
| Initialization | Same for all | Different per strategy |
| Copy-able | Yes | Usually no |

### Anti-Pattern: Scattered Conditionals

```rust
// Bad: conditionals scattered everywhere
if provider == "anthropic" {
    call_anthropic(...)
} else {
    call_openai(...)
}

// Later in another function
let model = if provider == "anthropic" { "claude" } else { "gpt-4" };
```

---

## 5. Complexity Management

### Prefer Early Returns

Reduce nesting by returning early on error conditions.

```rust
// Good: flat structure, happy path at low indentation
pub fn invoke_tool(&self, name: &str, params: Value) -> Result<Value> {
    let tool = self.tools.get(name)
        .ok_or(Error::ToolNotFound(name.to_string()))?;
    
    if !self.policy.allows(&tool.capabilities) {
        return Err(Error::CapabilityDenied(name.to_string()));
    }
    
    let result = tool.execute(params)?;
    
    // Happy path continues here
    Ok(result)
}

// Bad: arrow code, logic buried deep
pub fn invoke_tool(&self, name: &str, params: Value) -> Result<Value> {
    if let Some(tool) = self.tools.get(name) {
        if self.policy.allows(&tool.capabilities) {
            if let Ok(result) = tool.execute(params) {
                // Happy path buried 3 levels deep
                return Ok(result);
            }
        }
    }
    Err(Error::Failed)
}
```

### Function Extraction

Extract a function when:
- A code block has a clear, nameable purpose
- The same logic is used more than once
- Testing would benefit from isolation

### Module Organization

Organize by responsibility, not technical layer.

```rust
// Good: clear responsibilities
mod session;      // Session management
mod run;          // Run execution  
mod event;        // Event handling

// Bad: vague groupings
mod utils;
mod helpers;
mod common;
```

---

## 6. Error Handling

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("session not found: {0}")]
    SessionNotFound(SessionId),

    #[error("capability denied: {capability} for {tool}")]
    CapabilityDenied { capability: String, tool: String },

    #[error("tool timeout after {0:?}")]
    ToolTimeout(Duration),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Descriptive Error Messages

```rust
// Good: actionable information
return Err(Error::InvalidConfig(format!(
    "max_tokens {} exceeds model limit {}", 
    config.max_tokens, 
    model.limit
)));

// Bad: unhelpful
return Err(Error::InvalidConfig("bad value".to_string()));
```

### When to Panic

Almost never. Panics are for programmer errors that should never happen.

```rust
// Acceptable: exhaustive enum
match kind {
    Kind::A => 1,
    Kind::B => 2,
    _ => unreachable!(),
}

// Unacceptable: recoverable error
let value = map.get(key).expect("key must exist");  // Use Result
```

---

## 7. Concurrency

### Prefer Message Passing

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel(32);

// Producer
tx.send(Event::ToolInvoked { name: "read".into() }).await?;

// Consumer
while let Some(event) = rx.recv().await {
    handle_event(event).await;
}
```

### Minimize Shared State

```rust
// Keep critical sections small
let count = {
    let guard = self.counter.lock().unwrap();
    *guard
}; // Lock released here

// Better for simple counters: atomics
use std::sync::atomic::{AtomicU64, Ordering};
counter.fetch_add(1, Ordering::SeqCst);
```

### Async Patterns

```rust
use tokio::sync::{RwLock, Semaphore};

// Read-heavy: RwLock
let cache: Arc<RwLock<HashMap<K, V>>> = Arc::new(RwLock::new(HashMap::new()));

// Rate limiting: Semaphore
let permits = Arc::new(Semaphore::new(10));
let permit = permits.acquire().await?;
```

---

## 8. Documentation

### What to Document

- Brief purpose of the item
- Non-obvious constraints
- Important assumptions
- Complex usage patterns (with examples)

### What to Skip

- Obvious argument descriptions (names should be self-documenting)
- Error cases that must be handled anyway
- Redundant information clear from types

### Good Example

```rust
/// Invoke a tool with the given parameters.
///
/// The tool must be registered and the session must have the required capabilities.
pub fn invoke(&self, name: &str, params: Value) -> Result<Value>
```

### Bad Example (Too Verbose)

```rust
/// Invoke a tool with the given parameters.
///
/// # Arguments
/// * `name` - The tool name
/// * `params` - The parameters as JSON
///
/// # Errors
/// Returns Error::ToolNotFound if tool doesn't exist.
```

### Use `#![warn(missing_docs)]`

```rust
// In lib.rs
#![warn(missing_docs)]
```

---

## 9. Testing

### Unit Tests in Same File

```rust
pub fn parse_duration(s: &str) -> Result<Duration> {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_valid_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn parse_duration_invalid_returns_error() {
        assert!(parse_duration("invalid").is_err());
    }
}
```

### Test Naming

```rust
#[test]
fn <function>_<scenario>_<expected>() { }

// Examples:
fn parse_config_valid_returns_config()
fn parse_config_missing_field_returns_error()
fn invoke_tool_timeout_returns_timeout_error()
```

### Integration Tests in `/tests`

```
tests/
  integration_test.rs
  common/
    mod.rs  # Shared utilities
```

---

## 10. Project Structure

### Crate Layout

```
crates/
  runtime/
    src/
      lib.rs          # Public API, re-exports
      session.rs
      run.rs
      error.rs
    Cargo.toml
  storage/
    src/
      lib.rs
      sqlite.rs
    Cargo.toml
```

### Module Organization

```rust
// lib.rs - define public API
mod session;
mod run;
mod error;

pub use session::{Session, SessionId};
pub use run::{Run, RunId};
pub use error::{Error, Result};
```

---

## 11. Anti-Patterns

### Boolean Blindness

```rust
// Bad
fn create_session(user_id: &str, is_persistent: bool)

// Good  
fn create_session(user_id: &str, persistence: Persistence)

enum Persistence { Ephemeral, Persistent }
```

### Stringly-Typed Code

```rust
// Bad
fn set_model(&mut self, model: &str)  // "sonnet" | "opus" ???

// Good
fn set_model(&mut self, model: Model)

enum Model { Sonnet, Opus }
```

### Deep Nesting (Arrow Code)

See "Prefer Early Returns" above.

### Premature Abstraction

```rust
// Bad: over-engineered before requirements are clear
trait MessageHandler<T, E> {
    type Config;
    fn handle<W: Write>(&self, w: W, cfg: Self::Config) -> Result<(), E>;
}

// Good: concrete implementation first
pub struct Session { /* ... */ }
impl Session {
    pub fn handle_message(&mut self, msg: Message) -> Result<()> { /* ... */ }
}
// Extract abstractions when multiple implementations emerge
```

---

## 12. Code Review Checklist

- [ ] Public APIs have documentation
- [ ] Error cases return `Result` (not `panic!`)
- [ ] Types represent domain concepts clearly
- [ ] Functions have single, clear responsibilities
- [ ] No redundant or obvious comments
- [ ] Early returns preferred over deep nesting
- [ ] Strategy pattern used for varying behavior
- [ ] Test coverage for new functionality
- [ ] Breaking changes are documented

---

## References

- [The Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [ripgrep](https://github.com/BurntSushi/ripgrep) — exemplary project
- [tokio](https://github.com/tokio-rs/tokio) — async patterns
- [thiserror](https://github.com/dtolnay/thiserror) — error handling

---

*Style is about consistency. When in doubt, match surrounding code.*
