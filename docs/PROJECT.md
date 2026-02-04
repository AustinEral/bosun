# Bosun Project Structure

This document provides a comprehensive overview of the Bosun codebase architecture.

## Overview

Bosun is a local-first AI agent runtime written in Rust. It provides:
- A CLI for interactive chat sessions
- SQLite-based event persistence
- Capability-based security policy
- MCP (Model Context Protocol) tool integration
- Pluggable LLM backends

## Repository Layout

```
bosun/
├── Cargo.toml              # Workspace root
├── bosun.example.toml      # Example configuration file
├── crates/
│   ├── cli/                # CLI binary
│   ├── runtime/            # Core session & LLM logic
│   ├── storage/            # SQLite event store
│   ├── policy/             # Capability-based security
│   └── mcp/                # MCP client library
└── docs/
    ├── PROJECT.md          # This file
    ├── SPEC.md             # Detailed specification
    ├── ARCHITECTURE.md     # Architecture decisions
    ├── STYLE.md            # Code style guide
    ├── VISION.md           # Project vision
    └── COMMUNITY.md        # Community guidelines
```

## Crates

### `cli` — Command-Line Interface

**Path:** `crates/cli/`

The main entry point. Provides the `bosun` binary with subcommands for chat, session listing, and log viewing.

**Key files:**
- `main.rs` — CLI entry point, argument parsing (clap), chat loop
- `config.rs` — TOML configuration loading (`bosun.toml`)
- `error.rs` — CLI-specific error types

**Dependencies:** runtime, storage, policy

**Responsibilities:**
- Parse CLI arguments
- Load configuration from `bosun.toml`
- Initialize the LLM backend
- Run the interactive chat loop
- Display session history and logs

---

### `runtime` — Core Runtime

**Path:** `crates/runtime/`

The heart of Bosun. Manages sessions, LLM communication, and coordinates between components.

**Key files:**
- `lib.rs` — Public exports
- `session.rs` — `Session<B: LlmBackend>` struct, conversation state, message handling
- `backend/mod.rs` — `LlmBackend` trait definition
- `backend/anthropic.rs` — Anthropic API implementation with OAuth support
- `llm.rs` — Legacy client code (to be removed)
- `error.rs` — Runtime error types

**Key types:**
```rust
// The LLM backend trait (async, static dispatch)
pub trait LlmBackend: Send + Sync {
    fn chat(&self, request: ChatRequest<'_>) 
        -> impl Future<Output = Result<ChatResponse>> + Send;
}

// A conversation session, generic over backend
pub struct Session<B: LlmBackend> {
    pub id: SessionId,
    store: EventStore,
    backend: B,
    policy: Policy,
    messages: Vec<Message>,
    system: Option<String>,
}
```

**Responsibilities:**
- Abstract LLM providers behind a trait
- Manage conversation state
- Persist events to storage
- Enforce capability policy

---

### `storage` — Event Persistence

**Path:** `crates/storage/`

SQLite-based append-only event log. All session activity is recorded as immutable events.

**Key files:**
- `lib.rs` — Public exports
- `store.rs` — `EventStore` struct, SQLite operations
- `event.rs` — `Event`, `EventKind`, `Role`, `SessionId` types
- `error.rs` — Storage error types

**Key types:**
```rust
pub enum EventKind {
    SessionStart,
    SessionEnd,
    Message { role: Role, content: String },
    ToolCall { name: String, input: Value },
    ToolResult { name: String, output: Value },
}

pub struct Event {
    pub id: Uuid,
    pub session_id: SessionId,
    pub timestamp: DateTime<Utc>,
    pub kind: EventKind,
}
```

**Responsibilities:**
- Create and manage SQLite database
- Append events (immutable log)
- Query sessions and events
- Provide session summaries

---

### `policy` — Capability Security

**Path:** `crates/policy/`

Capability-based security system. All side effects require explicit permission.

**Key files:**
- `lib.rs` — Public exports
- `policy.rs` — `Policy` struct, TOML parsing, rule checking
- `capability.rs` — `CapabilityKind`, `CapabilityRequest` types
- `error.rs` — Policy error types

**Key types:**
```rust
pub enum CapabilityKind {
    FsRead,
    FsWrite,
    NetHttp,
    Exec,
    SecretsRead,
}

pub enum Decision {
    Allow,
    Deny { reason: String },
}
```

**Configuration (in `bosun.toml`):**
```toml
[allow]
fs_read = [".", "./src/**"]
fs_write = ["."]
net_http = ["api.anthropic.com"]
exec = ["git", "cargo"]

[deny]
all = ["secrets_read"]
```

**Responsibilities:**
- Parse policy from TOML
- Check capability requests against rules
- Support glob patterns for paths
- Provide default restrictive policy

---

### `mcp` — Model Context Protocol Client

**Path:** `crates/mcp/`

Client library for MCP servers. Spawns servers as subprocesses and communicates via JSON-RPC over stdio.

**Key files:**
- `lib.rs` — Public exports, usage example
- `server.rs` — `Server` struct, spawn, initialize, tool calls
- `protocol.rs` — JSON-RPC types, MCP message schemas
- `error.rs` — MCP error types

**Key types:**
```rust
pub struct ServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

pub struct Server {
    // Manages subprocess lifecycle
    // Provides tool listing and invocation
}

pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}
```

**Responsibilities:**
- Spawn MCP server processes
- Handle JSON-RPC protocol
- List available tools
- Invoke tools with arguments
- Manage server lifecycle

---

## Data Flow

```
User Input
    │
    ▼
┌─────────┐     ┌─────────┐     ┌───────────┐
│   CLI   │────▶│ Runtime │────▶│  Backend  │
│         │     │ Session │     │(Anthropic)│
└─────────┘     └────┬────┘     └───────────┘
                     │
         ┌───────────┼───────────┐
         ▼           ▼           ▼
    ┌─────────┐ ┌─────────┐ ┌─────────┐
    │ Storage │ │ Policy  │ │   MCP   │
    │ (Event) │ │ (Check) │ │ (Tools) │
    └─────────┘ └─────────┘ └─────────┘
```

1. **CLI** receives user input
2. **Session** adds message to conversation
3. **Storage** persists the event
4. **Backend** sends request to LLM
5. **Policy** checks any tool call capabilities
6. **MCP** executes approved tool calls
7. **Storage** persists tool events
8. Response flows back to user

---

## Configuration

Bosun uses a single `bosun.toml` file for all configuration:

```toml
# Backend settings
[backend]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key = "sk-ant-..."  # Optional, falls back to env var

# Security policy
[allow]
fs_read = ["."]
fs_write = ["."]

[deny]
all = ["exec", "net_http"]
```

**Load order:**
1. `./bosun.toml` if present
2. Default restrictive config if not

**API key resolution:**
1. `backend.api_key` in config
2. `ANTHROPIC_API_KEY` environment variable

---

## Building & Running

```bash
# Build all crates
cargo build

# Run the CLI
cargo run -p cli

# Or after install
bosun chat

# Run tests
cargo test

# Check formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

---

## Adding a New Backend

1. Create a new file in `crates/runtime/src/backend/`
2. Implement the `LlmBackend` trait
3. Export from `backend/mod.rs`
4. Add provider matching in CLI config

```rust
pub struct MyBackend { /* ... */ }

impl LlmBackend for MyBackend {
    async fn chat(&self, request: ChatRequest<'_>) -> Result<ChatResponse> {
        // Implementation
    }
}
```

---

## Adding a New Capability

1. Add variant to `CapabilityKind` in `crates/policy/src/capability.rs`
2. Add allowlist field to `AllowRules` in `policy.rs`
3. Add check logic in `Policy::check()`
4. Document in `bosun.example.toml`

---

## Key Design Decisions

- **Static dispatch for backends** — `Session<B>` avoids vtable overhead
- **Native async traits** — Requires Rust 1.75+, no `async-trait` crate
- **Append-only event log** — All state changes are events
- **Capability-based security** — Deny by default, explicit allow
- **MCP for tools** — Standard protocol, language-agnostic servers
