# Bosun — Vision

## One-liner

*Your AI agent. Your device. Your rules.*

## The Problem

Current agent frameworks are:

| Problem | Impact |
|---------|--------|
| **Heavy** | Require bulky runtimes, server setups, significant RAM |
| **Expensive** | Poor context discipline, weak cost awareness |
| **Hard to extend** | Extensions coupled to framework internals |
| **Fragile memory** | Chat logs grow; retrieval becomes messy |
| **Unsafe by default** | Side effects easy to trigger accidentally |
| **Developer-only** | Non-devs struggle to run them reliably |

**Result:** Personal agents don't feel like durable tools.

## What We're Building

A lightweight, local-first AI agent runtime that:

1. **Runs anywhere** — Desktop, server, Raspberry Pi. Later: mobile, browser.
2. **Stays light** — Single binary, minimal memory, fast start.
3. **Remembers usefully** — Pinned profile + search in v0.x; smarter memory later.
4. **Spends tokens wisely** — Hard budgets, cost tracking, smart context.
5. **Extends easily** — MCP tools work out of the box, any language.
6. **Stays private** — Data local by default, no telemetry.

## Core Principle

> **All side effects require an explicit capability.**

This anchors everything. Tools, network, filesystem, secrets — no capability, no access.

## Design Principles

| Principle | Meaning |
|-----------|---------|
| **Capability-first** | Side effects require explicit permission |
| **Community-first** | Reuse existing tool ecosystems (MCP) |
| **Local-first** | Data stays on device by default |
| **Small core** | Runtime stays slim; extensions are external |
| **Cost-aware** | Hard budgets + tracking |
| **Observable** | Everything debuggable via event log |
| **Staged portability** | Desktop first; mobile/browser later |

## Target Users

| Version | Target | Interface |
|---------|--------|-----------|
| v0.x | Power users, developers | CLI + config files |
| v1.x | Technical users | CLI + HTTP API + TUI |
| v2.x | General users | GUI shell, mobile apps |

## What It's NOT

- Not a cloud service (self-hosted only)
- Not multi-user (single-user focus)
- Not a model trainer (uses existing LLMs)
- Not trying to replace OpenClaw (different niche — lighter, more portable)

## The Bet

We're betting that:

1. People want agents they control, on devices they own
2. Lightweight + efficient beats feature-heavy + bloated
3. MCP ecosystem provides tools; we provide the runtime
4. The market for "personal agent runtime" is underserved

## Success Looks Like

- Useful agent running on a $35 Raspberry Pi
- Day-one access to hundreds of tools via MCP
- Monthly API costs under $10 for casual use
- "Why did it do that?" always answerable via logs
