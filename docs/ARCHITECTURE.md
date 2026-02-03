# Bosun — Architecture Reference

> **Status:** Future reference (post v0.x)
> 
> **Source of truth for v0.x:** See [SPEC.md](./SPEC.md)
>
> This document describes the full architecture vision. Most of this is **deferred** until after M0/M1 ship. Read SPEC.md for what we're actually building now.

---

## Document Purpose

This is a reference for future development phases. It captures:
- Where we're headed long-term
- Design decisions we've made for later
- Technical details that aren't needed yet

**Do not use this as a v0.x implementation guide.** Use SPEC.md.

---

## Core Principle (All Versions)

> **All side effects require an explicit capability.**

This anchors everything. Tools, network, filesystem, secrets — no capability, no access.

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
