# Bosun — Rust Style Guide

A practical style guide for Bosun development. Combines official Rust guidelines with lessons from excellent projects like ripgrep, tokio, and axum.

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

// Good: one blank line between functions
fn foo() {
    // ...
}

fn bar() {
    // ...
}
```

### Imports

Group and sort imports:

```rust
// 1. std library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// 3. Internal crates/modules
use crate::config::Config;
use crate::error::Result;
```

---

## 2. Naming

Follow [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html).

| Item | Convention | Example |
|------|------------|---------|
| Crates | `snake_case` | `bosun_runtime` |
| Modules | `snake_case` | `event_store` |
| Types (struct, enum, trait) | `UpperCamelCase` | `SessionManager` |
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

### Getters

```rust
// Good: no get_ prefix
fn name(&self) -> &str
fn is_empty(&self) -> bool
fn has_children(&self) -> bool

// Bad
fn get_name(&self) -> &str
```

### Iterators

```rust
// Good: iter, iter_mut, into_iter
fn iter(&self) -> Iter<'_, T>
fn iter_mut(&mut self) -> IterMut<'_, T>
fn into_iter(self) -> IntoIter<T>
```

---

## 3. Types & Structures

### Newtypes for Clarity

```rust
// Good: distinct types prevent mixing up IDs
struct SessionId(Ulid);
struct RunId(Ulid);

// Bad: easy to confuse
fn start_run(session: Ulid, run: Ulid);

// Good: types make it clear
fn start_run(session: SessionId, run: RunId);
```

### Builder Pattern for Complex Construction

```rust
// Good: builder for types with many optional fields
let config = ConfigBuilder::new()
    .model("claude-sonnet-4-20250514")
    .max_tokens(8000)
    .timeout(Duration::from_secs(30))
    .build()?;

// Implementation
pub struct ConfigBuilder {
    model: Option<String>,
    max_tokens: Option<usize>,
    timeout: Option<Duration>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn build(self) -> Result<Config> {
        Ok(Config {
            model: self.model.ok_or(Error::MissingField("model"))?,
            max_tokens: self.max_tokens.unwrap_or(4096),
            timeout: self.timeout.unwrap_or(Duration::from_secs(60)),
        })
    }
}
```

### Enums for State Machines

```rust
// Good: state is explicit and exhaustive
pub enum SessionState {
    Active,
    Paused,
    Waiting { for_event: EventType },
    Ended { reason: EndReason },
}

// Match forces handling all cases
match state {
    SessionState::Active => { /* ... */ }
    SessionState::Paused => { /* ... */ }
    SessionState::Waiting { for_event } => { /* ... */ }
    SessionState::Ended { reason } => { /* ... */ }
}
```

---

## 4. Error Handling

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("session not found: {0}")]
    SessionNotFound(SessionId),

    #[error("capability denied: {capability} for {tool}")]
    CapabilityDenied {
        capability: String,
        tool: String,
    },

    #[error("tool timeout after {0:?}")]
    ToolTimeout(Duration),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### Error Context

```rust
// Good: add context to errors
use anyhow::Context;

fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;

    toml::from_str(&content)
        .with_context(|| format!("failed to parse config: {}", path.display()))
}
```

### When to Panic

```rust
// OK to panic: programming errors, invariant violations
fn get_session(&self, id: SessionId) -> &Session {
    self.sessions.get(&id).expect("session must exist after creation")
}

// NOT OK: user input, external data, recoverable errors
// Use Result instead
fn parse_config(input: &str) -> Result<Config> {
    // ...
}
```

---

## 5. Concurrency

### Prefer Message Passing

```rust
// Good: channels for communication
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
// Good: Arc<Mutex<T>> only when necessary
// Keep critical sections small
let count = {
    let guard = self.counter.lock().unwrap();
    *guard
}; // Lock released here

// Better: use atomic types for simple counters
use std::sync::atomic::{AtomicU64, Ordering};

let counter = AtomicU64::new(0);
counter.fetch_add(1, Ordering::SeqCst);
```

### Async Patterns

```rust
// Good: use tokio's synchronization primitives
use tokio::sync::{RwLock, Semaphore};

// Read-heavy workloads: RwLock
let cache: Arc<RwLock<HashMap<K, V>>> = Arc::new(RwLock::new(HashMap::new()));

// Rate limiting: Semaphore
let permits = Arc::new(Semaphore::new(10));
let permit = permits.acquire().await?;
// do work
drop(permit); // release
```

---

## 6. Traits

### Implement Common Traits

Every public type should implement (where sensible):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Ulid);

// Also consider:
// - Default (if there's a sensible default)
// - Display (for user-facing output)
// - From/Into (for conversions)
```

### Trait Bounds

```rust
// Good: bounds on impl, not struct definition
pub struct Cache<K, V> {
    inner: HashMap<K, V>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash,
{
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }
}

// Bad: bounds on struct (forces bounds everywhere)
pub struct Cache<K: Eq + Hash, V> {
    inner: HashMap<K, V>,
}
```

---

## 7. Documentation

### Every Public Item

```rust
/// A session represents a conversation context.
///
/// Sessions execute runs serially and maintain working memory
/// across interactions.
///
/// # Examples
///
/// ```
/// let session = Session::new(channel_id);
/// let run = session.start_run()?;
/// ```
pub struct Session {
    // ...
}
```

### Document Errors and Panics

```rust
/// Invokes a tool by name with the given parameters.
///
/// # Arguments
///
/// * `name` - The tool name as registered
/// * `params` - JSON parameters for the tool
///
/// # Errors
///
/// Returns `Error::ToolNotFound` if the tool doesn't exist.
/// Returns `Error::CapabilityDenied` if the capability check fails.
/// Returns `Error::ToolTimeout` if the tool exceeds its timeout.
///
/// # Panics
///
/// Panics if the runtime is not initialized.
pub async fn invoke(&self, name: &str, params: Value) -> Result<Value> {
    // ...
}
```

### Use `#![warn(missing_docs)]`

```rust
// In lib.rs
#![warn(missing_docs)]
```

---

## 8. Testing

### Unit Tests in Same File

```rust
pub fn parse_duration(s: &str) -> Result<Duration> {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("invalid").is_err());
    }
}
```

### Integration Tests in `/tests`

```
tests/
  integration_test.rs
  common/
    mod.rs  # shared test utilities
```

### Test Naming

```rust
#[test]
fn test_<function>_<scenario>_<expected>() {
    // ...
}

// Examples:
fn test_parse_config_valid_returns_config()
fn test_parse_config_missing_field_returns_error()
fn test_invoke_tool_timeout_returns_timeout_error()
```

---

## 9. Project Structure

### Crate Organization

```
crates/
  runtime/
    src/
      lib.rs          # Public API, re-exports
      session.rs      # Session management
      run.rs          # Run execution
      error.rs        # Error types
    Cargo.toml

  storage/
    src/
      lib.rs
      sqlite.rs       # SQLite backend
      schema.rs       # Database schema
    Cargo.toml
```

### Module Organization

```rust
// lib.rs - define public API
mod session;
mod run;
mod error;

pub use session::{Session, SessionId, SessionState};
pub use run::{Run, RunId};
pub use error::{Error, Result};
```

### Keep Modules Focused

- One primary responsibility per module
- If a module grows >500 lines, consider splitting
- Prefer many small modules over few large ones

---

## 10. Performance Considerations

### Avoid Unnecessary Allocations

```rust
// Good: return reference when possible
fn name(&self) -> &str {
    &self.name
}

// Good: accept references or impl traits
fn process(data: &[u8]) { /* ... */ }
fn process(data: impl AsRef<[u8]>) { /* ... */ }

// Avoid: unnecessary clone
fn process(data: Vec<u8>) { /* ... */ }  // Forces clone at call site
```

### Use Iterators

```rust
// Good: lazy, no intermediate allocation
let sum: u64 = events
    .iter()
    .filter(|e| e.kind == EventKind::ToolInvoked)
    .map(|e| e.duration_ms)
    .sum();

// Avoid: intermediate collections
let filtered: Vec<_> = events.iter().filter(/* ... */).collect();
let sum: u64 = filtered.iter().map(/* ... */).sum();
```

### Benchmark Before Optimizing

```rust
// In benches/benchmark.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_parse_config(c: &mut Criterion) {
    let input = include_str!("../fixtures/config.toml");
    c.bench_function("parse_config", |b| {
        b.iter(|| parse_config(input))
    });
}

criterion_group!(benches, bench_parse_config);
criterion_main!(benches);
```

---

## References

- [The Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [ripgrep](https://github.com/BurntSushi/ripgrep) — exemplary project structure
- [tokio](https://github.com/tokio-rs/tokio) — async patterns
- [thiserror](https://github.com/dtolnay/thiserror) — error handling

---

*Style is about consistency. When in doubt, match the surrounding code.*
