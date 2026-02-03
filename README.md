# Bosun

*A lightweight, local-first AI agent runtime. Your device. Your rules.*

## Status

**Phase:** Planning / Pre-development  
**Date:** 2026-02-03

## Quick Summary

**What:** Rust-based AI agent runtime that's lightweight, private, and cost-aware.

**Why:** Current agent frameworks are heavy, expensive, and developer-only. We want something that runs on a Pi, respects your wallet, and leverages existing tools.

**How:** 
- Small Rust core with capability-based security
- MCP-first tool integration (borrow from community)
- SQLite storage with event logging
- Token budgets and cost tracking built-in

## Core Principle

> *All side effects require an explicit capability.*

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/VISION.md](./docs/VISION.md) | Why we're building this |
| [docs/SPEC.md](./docs/SPEC.md) | What to build (M0/M1 scope) |
| [docs/STYLE.md](./docs/STYLE.md) | How we write code |
| [docs/COMMUNITY.md](./docs/COMMUNITY.md) | MCP-first tool strategy |
| [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) | Future architecture reference |

## Contributing

- **Humans:** See [CONTRIBUTING.md](./CONTRIBUTING.md)
- **AI Agents:** See [AGENTS.md](./AGENTS.md)

## Milestones

### M0: Core Loop
- CLI chat
- SQLite event log
- Capability gate + policy
- LLM streaming (Anthropic)
- Safe defaults

### M1: Community Tools
- MCP client integration
- Tool listing + invocation
- Capability enforcement at tool boundary
- Cost tracking

### M2+: Future
- Memory system (pinned facts + search)
- HTTP API
- Context budgets + compression
- Portability (WASM, mobile)

## Getting Started

*Not yet implemented. This is planning documentation.*

```bash
# Future usage
agent init
agent chat
```

## License

Apache 2.0 — see [LICENSE](./LICENSE)
