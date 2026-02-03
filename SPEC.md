# Bosun — Technical Specification (M0/M1)

**Version:** 0.2
**Date:** 2026-02-03
**Project:** Bosun

---

## 1. Scope

### M0: Core Loop

Deliver a working local agent via CLI:

- Sessions + serialized run queue
- LLM streaming response
- SQLite-backed event log (debuggable)
- Capability gate + policy file (TOML)
- Safe defaults (no exec by default)

**Exit criteria:** Can chat, see runs/events, runtime denies side effects unless allowed.

### M1: Community Tools via MCP

Add community leverage:

- MCP client (connect to MCP servers)
- Tool listing + invocation
- Capability checks at tool boundary
- Timeouts + basic schema checks
- Cost tracking per run/session

**Exit criteria:** Can connect existing MCP tool servers and use them safely.

---

## 2. Non-Goals (M0/M1)

- Browser/WASM runtime
- iOS/Android integration
- WASM tool sandboxing
- Multi-agent networking
- Multi-language tool SDKs
- Auto-updates/signing
- Advanced memory (entities, decay, consolidation)
- Automatic model routing

---

## 3. Runtime Behavior

### 3.1 Session Model

- A **session** is a stable conversation context
- Sessions execute runs **serially** (one run at a time)
- A run emits events: lifecycle → assistant deltas → tool events

### 3.2 Run Loop

1. Accept user input
2. Load policy + capabilities
3. Assemble context under token budget
4. Call LLM (streaming)
5. If tool call:
   - Validate tool name/params
   - Enforce capability checks
   - Invoke tool (MCP)
   - Return result to model
6. Emit run finished (success/error)
7. Persist events

---

## 4. Capabilities & Policy

### 4.1 Capability Set

| Capability | Description | Default |
|------------|-------------|---------|
| `fs_read` | Read files within roots | Workspace only |
| `fs_write` | Write files within roots | Workspace only |
| `net_http` | HTTP requests to domains | Allowlist |
| `exec` | Execute commands | **Denied** |
| `secrets_read` | Access secret keys | Allowlist |

### 4.2 Policy Rules

- Default-deny for `exec`
- Default workspace scoping for FS
- Allowlist for network domains
- All policy decisions logged as events

---

## 5. Storage Schema (SQLite)

### 5.1 Tables

```sql
-- Sessions: persistent chat threads
CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  title TEXT
);

-- Runs: each user message triggers one run
CREATE TABLE IF NOT EXISTS runs (
  run_id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  status TEXT NOT NULL,         -- running | succeeded | failed
  model TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  cost_usd REAL,
  error TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(session_id)
);

-- Events: append-only timeline
CREATE TABLE IF NOT EXISTS events (
  event_id TEXT PRIMARY KEY,
  session_id TEXT,
  run_id TEXT,
  ts INTEGER NOT NULL,
  kind TEXT NOT NULL,
  data_json TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES sessions(session_id),
  FOREIGN KEY(run_id) REFERENCES runs(run_id)
);

CREATE INDEX IF NOT EXISTS idx_events_session_ts ON events(session_id, ts);
CREATE INDEX IF NOT EXISTS idx_events_run_ts ON events(run_id, ts);
```

### 5.2 Event Kinds

```
session.created
run.started
prompt.built          # metadata only, no secrets
assistant.delta       # stream chunks
assistant.message     # final message
tool.requested
tool.invoked
tool.succeeded
tool.failed
capability.granted
capability.denied
run.succeeded
run.failed
```

---

## 6. Configuration

### 6.1 agent.toml

```toml
[storage]
path = "./agent.db"

[workspace]
root = "./workspace"

[llm]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
max_context_tokens = 8000
max_response_tokens = 1200

[policy]
allow_exec = false
allow_network = true

[policy.fs]
read_roots = ["./workspace"]
write_roots = ["./workspace"]

[policy.net_http]
allow_domains = ["api.github.com"]

[secrets]
keys = ["ANTHROPIC_API_KEY"]

# MCP servers (M1)
[[mcp_servers]]
name = "filesystem"
command = "mcp-filesystem"
args = ["--root", "./workspace"]

[[mcp_servers]]
name = "github"
command = "mcp-github"
args = []
```

---

## 7. CLI Commands

```bash
agent init                 # Create agent.toml + workspace
agent chat                 # Interactive chat
agent sessions             # List sessions
agent logs --session <id>  # Show session events
agent logs --run <id>      # Show run events
agent mcp list             # List MCP servers + tools (M1)
agent mcp call <tool>      # Invoke tool directly (M1)
```

---

## 8. Tool System (M1)

### 8.1 MCP Integration

- Connect to configured MCP servers (stdio JSON-RPC)
- Request tool list on connect
- Expose tools to model (name + schema)
- On tool call:
  - Validate args (basic shape check)
  - Enforce capability policy
  - Invoke with timeout
  - Log result summary

### 8.2 Limits

| Limit | Default |
|-------|---------|
| Tool timeout | 15s |
| Tool output max | 256KB |

Truncate + log if exceeded.

---

## 9. Acceptance Checklist

### M0

- [ ] CLI chat works
- [ ] Events persisted and viewable
- [ ] Runs show token usage + cost
- [ ] Side effects denied by default
- [ ] Policy file loaded and respected

### M1

- [ ] MCP servers connect reliably
- [ ] Tools list/invoke works
- [ ] Capability checks block unsafe calls
- [ ] Tool timeouts handled cleanly
- [ ] Tool calls logged with summaries
- [ ] Cost tracking per session

---

## 10. Crate Layout

```
crates/
  runtime/        # Session loop, runs, events
  storage/        # SQLite
  policy/         # Capability + tool policy
  llm/            # Adapter traits + Anthropic impl
  mcp/            # MCP client
  tools/          # Tool routing + boundary checks
  cli/            # CLI channel
```
