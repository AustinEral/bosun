# Contributing to Bosun

## Golden Rule

**All changes go through pull requests.** Never commit directly to `main`.

---

## Workflow

### 1. Create a Branch

```bash
git checkout main
git pull origin main
git checkout -b <type>/<short-description>
```

**Branch naming:**
- `feat/` — New feature
- `fix/` — Bug fix
- `refactor/` — Code restructuring (no behavior change)
- `docs/` — Documentation only
- `test/` — Test additions or fixes
- `chore/` — Build, CI, dependencies

Examples:
- `feat/mcp-client`
- `fix/session-timeout`
- `refactor/event-store`
- `docs/api-guide`

### 2. Make Changes

- Follow [STYLE.md](./STYLE.md)
- Write tests for new functionality
- Run checks locally before pushing:

```bash
cargo fmt --check
cargo clippy
cargo test
```

### 3. Commit

**Commit message format:**
```
<type>: <short summary>

<optional body explaining why, not what>

<optional footer with breaking changes, issue refs>
```

**Types:** `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

**Examples:**
```
feat: add MCP client connection

Implements stdio JSON-RPC connection to MCP servers.
Tool listing and invocation working.

Closes #12
```

```
fix: handle tool timeout correctly

Was silently dropping errors. Now surfaces them
with proper context.
```

```
refactor: extract capability checking to separate module

No behavior change. Preparing for policy engine work.
```

### 4. Open a Pull Request

**PR title:** Same format as commit message (`type: short summary`)

**PR description should include:**
- What changed and why
- How to test (if not obvious)
- Breaking changes (if any)
- Screenshots/logs (if relevant)

**PR template:**
```markdown
## What

Brief description of the change.

## Why

Why is this change needed?

## How to Test

Steps to verify, or "covered by unit tests."

## Breaking Changes

None / List any breaking changes.
```

### 5. Review

- At least one approval required before merging
- Address review comments or discuss
- Keep PRs focused and reasonably sized
- Large changes should be split into smaller PRs

### 6. Merge

- Squash and merge (keeps main history clean)
- Delete branch after merge

---

## Before Opening a PR

### For Bug Fixes

- Confirm the bug exists on `main`
- Add a test that reproduces the bug
- Fix should make the test pass

### For New Features

- Check if there's an existing issue or discussion
- For large features, open an issue first to discuss approach
- Include tests and documentation

### For Refactors

- No behavior changes (tests should pass without modification)
- Explain the motivation in PR description

---

## Code Review Guidelines

**As an author:**
- Keep PRs small and focused
- Respond to feedback promptly
- Don't take feedback personally

**As a reviewer:**
- Be constructive and specific
- Approve if it's good enough, not perfect
- Use "nitpick:" prefix for optional suggestions

---

## Local Development

```bash
# Clone
git clone https://github.com/AustinEral/bosun.git
cd bosun

# Build
cargo build

# Test
cargo test

# Format
cargo fmt

# Lint
cargo clippy

# Docs
cargo doc --open
```

---

## Questions?

Open an issue or start a discussion.
