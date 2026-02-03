# Bosun

*A lightweight, local-first AI agent runtime.*

A lightweight, local-first AI agent runtime. Your device. Your rules.

## Status

**Phase:** Planning / Pre-development
**Date:** 2026-02-03

## Documentation

| Document | Purpose |
|----------|---------|
| [VISION.md](./VISION.md) | High-level vision, problem statement, principles |
| [SPEC.md](./SPEC.md) | Technical spec for M0/M1 milestones |
| [COMMUNITY.md](./COMMUNITY.md) | MCP-first community borrowing strategy |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Full architecture reference (future phases) |

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
