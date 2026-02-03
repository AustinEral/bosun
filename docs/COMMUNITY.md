# Bosun — Community Strategy (MCP-First)

## Strategy

Be the **best host** for existing tools: policy + logging + cost discipline + reliability.

Tools come from **MCP servers** built by the community.

---

## What We Reuse

### Yes — Leverage

- Existing MCP servers (filesystem, GitHub, Notion, Jira, etc.)
- Their tool schemas
- Their maintenance and ecosystems

### No — Don't Own Early

- A big built-in tool library
- Tool SDKs for every language
- A centralized cloud/registry service

---

## Compatibility Promise (v0.x)

### We Promise

- MCP stdio JSON-RPC connection
- Tool list discovery
- Tool invocation
- Basic schema validation
- Strong boundary controls (capability policy, timeouts, output limits)

### We Don't Promise

- Every MCP feature / vendor extension
- Perfect schema translation
- Tool sandboxing (yet)

---

## Tool Boundary Contract

Regardless of tool source, we enforce:

| Control | What It Does |
|---------|--------------|
| Capability check | No call without permission |
| Timeout | Hard limit, configurable |
| Output truncation | Prevent memory blowup |
| Structured logging | tool.requested → invoked → succeeded/failed |
| Deterministic errors | Clean failure surfaces |

**Tools can be messy. The runtime stays predictable.**

---

## Version Pinning (v0.x)

Simple approach:

- Users pin MCP server versions externally (package manager, container)
- Config pins command path + args
- We log server name/version if available

Later: lockfile + signed registries.

---

## Starter Pack (Docs, Not Code)

Ship documentation, not tools:

- Recommended MCP servers
- Safe policy templates per server
- Examples + troubleshooting

Gets adoption without becoming a distribution platform.

---

## Community Contributions That Help

### Yes — Valuable

- MCP server adapters for common apps
- "Safe default" policy templates
- Examples + docs
- Bug reports on boundary edge cases

### No — Avoid Pressure For

- Core runtime changes per tool
- Built-in tool additions
- Framework-specific SDKs early

---

## Security Posture

### Tools Are Untrusted By Default

Even community tools can be buggy or malicious.

So:
- Everything through capability policy
- Network is domain-limited
- Filesystem is root-limited
- Exec is off by default
- Secrets never logged

### Transparent Audit

"Why did it do that?" is always answerable:
- Run timeline
- Tool calls with summaries
- Capability grants/denials

---

## Starter Policy Template

Publish this day one:

```toml
[policy]
allow_exec = false
allow_network = true

[policy.fs]
read_roots = ["./workspace"]
write_roots = ["./workspace"]

[policy.net_http]
allow_domains = []  # User adds as needed
```

Community tools usable without making runtime scary.

---

## Roadmap

### v0.x (Now)

- MCP client support
- Safe boundary controls
- Documentation + policy templates

### v1.x (Later)

- HTTP API for community UIs/channels
- Tool pack conventions (naming, grouping, policy hints)

### v2.x (Future)

- WASM/WASI tools for sandboxing
- Signed tool packs / registry
- Multi-device sync
