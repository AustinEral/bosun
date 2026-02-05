# Bosun

*A lightweight, local-first AI agent runtime. Your device. Your rules.*

## Status

**Phase:** M0 Complete / Active Development  
**Current Work:** M1 (MCP tool integration)

## Quick Start

```bash
# Clone and build
git clone https://github.com/AustinEral/bosun.git
cd bosun
cargo build --release

# Start chatting
./target/release/bosun chat

# View session history
./target/release/bosun sessions

# View event logs
./target/release/bosun logs --session <id>
```

**Requirements:**
- Rust 1.75+ (for async trait support)
- `ANTHROPIC_API_KEY` environment variable

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

### M0: Core Loop ✓
- [x] CLI chat
- [x] SQLite event log
- [x] Capability gate + policy
- [x] LLM streaming (Anthropic)
- [x] Safe defaults

### M1: Community Tools (In Progress)
- [x] MCP client integration
- [x] Tool listing + invocation
- [ ] Capability enforcement at tool boundary
- [ ] Cost tracking per session

### M2+: Future
- Memory system (pinned facts + search)
- HTTP API
- Context budgets + compression
- Portability (WASM, mobile)

## Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run lints
cargo clippy
```

## License

Apache 2.0 — see [LICENSE](./LICENSE)
