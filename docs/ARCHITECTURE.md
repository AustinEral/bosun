# Bosun — Architecture Reference

> **Status:** Living document
> 
> **Source of truth for v0.x implementation:** See [SPEC.md](./SPEC.md)
>
> This document describes the full architecture vision and design philosophy. Some sections are deferred until after M0/M1 ship. Read SPEC.md for current implementation scope.

---

## Document Purpose

This captures:
- Core design philosophy
- Where we're headed long-term
- How components fit together
- Design decisions and rationale

**For v0.x implementation details:** Use SPEC.md.

---

## Core Principle (All Versions)

> **All side effects require an explicit capability.**

This anchors everything. Tools, network, filesystem, secrets — no capability, no access.

---

## Design Philosophy: Runtime, Not Framework

Bosun is a **runtime**, not a framework. This distinction shapes everything.

| Framework Approach | Runtime Approach (Bosun) |
|--------------------|--------------------------|
| Tools built into the core | Tools provided by MCP servers (external) |
| Channels built into the core | Channels provided by adapters (external) |
| Plugins run in-process | Extensions run as separate processes |
| Add feature = grow the core | Add feature = new external component |
| One big binary with everything | Small core + composable pieces |

**Why this matters:**

1. **Composability** — Only run what you need. Raspberry Pi deployment? Just `bosun` + `bosun-telegram`. No Discord code loaded.

2. **Language-agnostic** — Channel adapters and MCP servers can be written in any language. They just speak the protocol.

3. **Replaceable** — Don't like our Telegram adapter? Write your own. The interface is documented.

4. **Debuggable** — Clear process boundaries. Each component has its own logs. No tangled in-process state.

5. **Lightweight core** — The runtime stays small. Complexity lives at the edges.

**What Bosun core provides:**
- Session/conversation management
- LLM communication
- MCP tool orchestration  
- Policy/capability enforcement
- Event logging
- API for channel adapters

**What lives outside the core:**
- Channel adapters (Telegram, Discord, CLI, etc.)
- MCP tool servers (filesystem, GitHub, web search, etc.)
- Future: Storage backends, memory providers

---

## Channel Adapter Architecture

Channels (Telegram, Discord, Slack, CLI, etc.) are **not** built into Bosun. They're separate binaries that communicate with the runtime via a simple API.

### Why Not MCP for Channels?

MCP is designed for tools — request-response patterns. Channels need:
- Persistent connections (WebSocket, long-polling)
- Incoming webhooks
- Platform-specific auth flows
- Bidirectional message flow

MCP doesn't fit. Channels need their own adapter pattern.

### Architecture Diagram

```
┌─────────────────┐     
│ bosun-telegram  │──┐     Adapters handle platform-specific
└─────────────────┘  │     concerns: auth, webhooks, formatting
┌─────────────────┐  │  ┌─────────────────┐     ┌─────────────────┐
│ bosun-discord   │──┼─▶│     Bosun       │────▶│   MCP Servers   │
└─────────────────┘  │  │    (runtime)    │     │    (tools)      │
┌─────────────────┐  │  └─────────────────┘     └─────────────────┘
│ bosun-cli       │──┘         │
└─────────────────┘            │
                               ▼
                        ┌─────────────┐
                        │   SQLite    │
                        │  (events)   │
                        └─────────────┘
```

### Adapter Responsibilities

Each channel adapter:
1. **Connects** to the platform (Telegram Bot API, Discord Gateway, etc.)
2. **Authenticates** using platform-specific credentials
3. **Receives** messages from the platform
4. **Forwards** messages to Bosun via API (HTTP or Unix socket)
5. **Receives** responses from Bosun
6. **Sends** responses back to the platform (with platform-specific formatting)

### Adapter Protocol (v1+)

*Deferred until v1.x. v0.x uses CLI only.*

The runtime will expose a simple API:

```
POST /v1/message
{
  "channel": "telegram",
  "channel_id": "12345",
  "user_id": "67890", 
  "content": "Hello Bosun",
  "metadata": { ... }
}

Response:
{
  "session_id": "abc-123",
  "response": "Hello! How can I help?",
  "metadata": { ... }
}
```

Adapters are stateless from Bosun's perspective. The runtime manages sessions.

### Example: Telegram Adapter

`bosun-telegram` would:
1. Read Telegram bot token from config
2. Connect to Telegram Bot API (long-polling or webhooks)
3. On incoming message → POST to Bosun API
4. On Bosun response → Send via Telegram API
5. Handle Telegram-specific features (inline keyboards, media, etc.)

The adapter is ~500-1000 lines. All the AI/conversation logic lives in Bosun.

### Benefits

- **Swap adapters** without touching the runtime
- **Multiple adapters** can connect simultaneously
- **Test adapters** independently
- **Community adapters** in any language
- **Platform-specific features** don't bloat the core

---

## User Experience by Version

The composable architecture is an **implementation detail**. Users see progressively simpler interfaces.

### v0.x: Developers

**Interface:** CLI + config files

```bash
# Manual composition
bosun chat                          # Interactive CLI
bosun --config ./bosun.toml chat    # With custom config
```

Users understand they're running separate processes. They edit TOML files. This is fine for developers.

### v1.x: Technical Users  

**Interface:** CLI + TUI + `bosun up`

```bash
# Orchestrated startup
bosun up                            # Reads bosun.toml, spawns everything
bosun up --with telegram,discord    # Explicit adapters
bosun status                        # Shows running components
bosun logs telegram                 # View adapter logs
```

```toml
# bosun.toml
[adapters]
telegram = { token_env = "TELEGRAM_BOT_TOKEN" }
discord = { token_env = "DISCORD_BOT_TOKEN" }

[[mcp_servers]]
name = "filesystem"
command = "mcp-filesystem"
args = ["--root", "./workspace"]
```

`bosun up` reads the config, spawns the runtime, spawns adapters, connects MCP servers. User runs one command.

### v2.x: General Users

**Interface:** GUI application

- **One installer** — Download Bosun.app (macOS), Bosun.exe (Windows), or bosun.deb (Linux)
- **Setup wizard** — "Connect to Telegram? [Scan QR code]"
- **Visual config** — Checkboxes for features, no TOML editing
- **System tray** — Runs in background, shows status
- **Hides complexity** — User doesn't know there are multiple processes

Under the hood, the GUI:
1. Manages `bosun.toml`
2. Runs `bosun up` equivalent
3. Monitors component health
4. Provides log viewer

**The architecture doesn't change** — we just add layers of UX on top.

### Distribution Strategy (v2.x)

| Platform | Distribution |
|----------|--------------|
| macOS | .dmg with signed .app bundle |
| Windows | .msi installer, optional winget |
| Linux | .deb, .rpm, Flatpak, AppImage |
| Raspberry Pi | .deb (arm64), setup script |

Installers bundle:
- Bosun runtime
- Common adapters (CLI, Telegram, Discord)
- GUI shell (v2.x)

Users can install additional adapters via:
```bash
bosun install adapter discord-voice
```

---

## Staged Portability

We take a phased approach to platforms:

| Phase | Platforms | Status |
|-------|-----------|--------|
| v0.x | Linux, macOS, Windows, Raspberry Pi | **Active** |
| v1.x | HTTP API enables community UIs | Planned |
| v2+ | Mobile (iOS/Android), Browser/WASM | Future (TBD ordering) |

**v0.x focuses on desktop/server/Pi.** Mobile and browser are explicitly deferred.

---

## v0.x Overview (For Orientation Only)

See [SPEC.md](./SPEC.md) for implementation details.

**Capability enforcement in v0.x:** Runtime gate at tool boundary + logged decisions. Type-level capability tokens are a later hardening step.

Summary:

```
┌───────────────────────────────────────────────┐
│               Core Runtime (Rust)             │
│  Sessions + Run Queue                         │
│  Context Assembly + Budgets                   │
│  Capability/Policy Gate                       │
│  Tool Router (MCP-first)                      │
│  Storage (SQLite) + Event Log                 │
│  LLM Adapter(s)                               │
└───────────────────────────────────────────────┘
          │                      │
          ▼                      ▼
   MCP Tool Servers        LLM Providers
 (community ecosystem)    (Anthropic, etc.)
```

---

## Future: Full Architecture (v1+)

*Everything below is deferred. Included for reference only.*

### Event Store (v1+)

v0.x uses simple SQLite event tables. Future enhancements:

- Hash-chaining for tamper-evidence (v1+)
- Cryptographic verification tools (v1+)
- Event compaction/archival (v1+)

### Memory System (v1+)

v0.x memory is simple:
- Transcript store (events are truth)
- Pinned user profile (structured facts/preferences)
- Keyword search (SQLite FTS)
- Optional embeddings behind feature flag

Future memory system (v1+/v2):

```rust
// FUTURE - Not v0.x
struct Entity {
    id: Ulid,
    entity_type: EntityType,  // Person, Place, Organization
    name: String,
    attributes: HashMap<String, Value>,
    relations: Vec<Relation>,
    provenance: Provenance,
}

struct Fact {
    id: Ulid,
    statement: String,
    confidence: f32,
    provenance: Provenance,
    decay_rate: f32,
}
```

Future capabilities:
- Entity graphs with relationships
- Confidence scoring and decay
- Conflict resolution
- Memory consolidation
- Multi-device sync

### Tool System Priority (v0.x vs Future)

**v0.x priority:**
1. MCP servers (stdio JSON-RPC) — **primary**
2. Simple stdio JSON tools (non-MCP) — optional

**Future priority (v1+):**
3. WASM tools (sandboxed, portable)
4. Native Rust tools (built-in only)

WASM sandboxing is a great future direction but not v0.x scope.

### Agent-to-Agent (v2+)

Entirely deferred. Future design notes:

- Explicit pairing (like Bluetooth/SSH)
- Capability delegation
- Encrypted transport (Noise/mTLS)
- Audit logging
- Local network only initially (no NAT traversal)

### Update & Signing (v1+)

v0.x: Manual updates, no signing required.

Future:
- Signed releases (ed25519)
- Optional auto-update
- Rollback support
- Tool/plugin version pinning

### Mobile & Browser (v2+)

Deferred entirely. Future considerations:

**Mobile (iOS/Android):**
- Static library + native wrapper
- App sandbox constraints
- No subprocess tools
- Battery/thermal limits for local models

**Browser (WASM):**
- IndexedDB/OPFS storage
- CORS networking restrictions
- No subprocess tools
- WASM-only tools

---

## Future Success Metrics

*Optimization goals for later, not v0.x requirements:*

| Metric | Target | Phase |
|--------|--------|-------|
| Binary size | <15MB | v1+ |
| Cold start | <500ms | v1+ |
| Idle memory | <30MB | v1+ |
| Active memory | <150MB | v1+ |
| WASM size | <5MB | v2+ |

v0.x metrics: "works reliably, cost tracking visible, logs are useful."

---

## Crate Layout (Full Vision)

v0.x crates (see SPEC.md):
```
crates/
  runtime/        # Session loop, runs, events
  storage/        # SQLite
  policy/         # Capability + tool policy
  llm/            # Adapter traits + Anthropic
  mcp/            # MCP client
  tools/          # Tool routing
  cli/            # CLI channel
```

Future crates (v1+):
```
crates/
  api/            # HTTP/WebSocket server
  memory/         # Advanced memory system
  wasm-host/      # WASM tool runtime
  sync/           # Multi-device sync
  network/        # Agent-to-agent protocol
```

---

## References

- [MCP Protocol Spec](https://modelcontextprotocol.io/)
- [WASI](https://wasi.dev/) (future)
- [wasmtime](https://wasmtime.dev/) (future)
- [Noise Protocol](https://noiseprotocol.org/) (future)

---

*This document will evolve as we ship v0.x and learn what's actually needed.*
