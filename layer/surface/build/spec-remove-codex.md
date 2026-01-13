---
id: spec-remove-codex
status: ready
created: 2026-01-13
tags: [spec, adapter, cleanup, architecture]
references: [adapter-pattern, unix-philosophy, spec-adapter-selection]
---

# Spec: Remove Codex from Adapters

**Problem:** Codex is in the adapter enum but doesn't fit the adapter model. It's a stub with no implementation and a conceptually different role.

**Solution:** Remove Codex from the adapter system. Three adapters remain: Claude Code, Gemini CLI, OpenCode.

---

## Core Values Alignment

### Adapter Pattern

From `layer/core/adapter-pattern.md`:

> "Use trait-based adapters to integrate with external systems (LLMs, databases, build tools) without coupling core logic to any specific implementation."

> "Common use cases in Patina: LLM adapters (Claude, Gemini, future providers)"

**An adapter is:**
- A CLI tool you launch with `patina` (e.g., `patina` → launches `claude`)
- Runs in project context with Patina integration
- Has MCP configuration, bootstrap file, templates
- Implements the adapter module pattern (`src/adapters/{name}/`)

**Codex doesn't fit:**
- No `src/adapters/codex/` implementation (unlike Claude, Gemini, OpenCode)
- `get_mcp_config()` returns `None` with `// TBD` marker
- No templates, no integration code
- Just a stub in the enum

### Unix Philosophy

From `layer/core/unix-philosophy.md`:

> "One tool, one job, done well."

Codex's job is different from adapters:
- **Adapters** = CLI tools patina launches in project context
- **Agents** = tools that adapters can spawn for specific tasks

Mixing these roles in the same enum violates single-responsibility. Codex should be in a separate agent system, not the adapter system.

### Dependable Rust

From `layer/core/dependable-rust.md`:

> "Keep your public interface small and stable."

The `Adapter` enum is a public interface. Including stubs (Codex with `// TBD` everywhere) bloats the interface with non-functional variants. Clean break: remove what isn't implemented.

---

## Adapter vs Agent

| Aspect | Adapter | Agent |
|--------|---------|-------|
| Relationship | Patina launches adapter | Adapter spawns agent |
| Context | Runs in project | Runs for specific task |
| Example | `patina` → Claude Code | Claude → Codex for research |
| Interface | Full module in `src/adapters/` | Future: `patina agent spawn` |
| Current | Claude, Gemini, OpenCode | (none implemented) |

Codex is an agent, not an adapter. Its current presence in the adapter enum was premature scaffolding.

---

## Current State

**Codex in codebase:**

| Location | Status |
|----------|--------|
| `src/adapters/codex/` | Does not exist |
| `ADAPTERS` const | Listed but no implementation |
| `Adapter::Codex` enum | Stub with match arms |
| `get_mcp_config()` | `None, // TBD` |
| Templates | None |

**Real adapters have:**
- `src/adapters/{name}/mod.rs` + `internal/`
- MCP configuration
- Template embedding
- Bootstrap generation

Codex has none of these.

---

## Changes Required

### `src/adapters/launch.rs`

```rust
// ADAPTERS const: remove "codex"
pub const ADAPTERS: &[&str] = &["claude", "gemini", "opencode"];

// Adapter enum: remove Codex variant
pub enum Adapter {
    Claude,
    Gemini,
    OpenCode,
}

// Remove from all match arms:
// - name()
// - display()
// - from_name()
// - bootstrap_file()
// - detect_commands()
// - get_mcp_config()
```

### `src/main.rs`

```rust
// Remove Llm::Codex variant and its match arm
pub enum Llm {
    Claude,
    Gemini,
    OpenCode,
}
```

### `layer/surface/build/spec-adapter-selection.md`

Update error messages and examples:
- Line 48: `Install one of: claude, gemini, opencode`
- Remove Codex from code examples throughout

### Tests

Update test assertions that reference Codex or adapter counts.

---

## Implementation Checklist

- [ ] Remove `Codex` from `Adapter` enum in `src/adapters/launch.rs`
- [ ] Remove `"codex"` from `ADAPTERS` const
- [ ] Remove all Codex match arms (6 functions)
- [ ] Update tests (remove assertions, adjust counts)
- [ ] Remove `Codex` from `Llm` enum in `src/main.rs`
- [ ] Update `spec-adapter-selection.md` references
- [ ] `cargo test && cargo clippy` - exhaustiveness check will catch misses

---

## Verification

```bash
# Should show only 3 adapters
patina adapter list

# Should error with clear message
patina --adapter=codex
# Error: Unknown adapter 'codex'. Valid adapters: claude, gemini, opencode

# Clean build
cargo build --release
cargo test --workspace
```

---

## Future: Agent System

Codex may return as part of an agent system:

```bash
# Future (out of scope)
patina agent list
patina agent spawn codex --task "research X"
```

This is a separate architectural concern. The agent system would:
- Have its own enum/registry
- Be invoked BY adapters, not AS adapters
- Have different lifecycle (task-scoped, not session-scoped)

Not designed here. This spec only removes the misplaced stub.

---

## Rejected Alternatives

### A. Keep Codex as placeholder for future

**Rejected:** Violates dependable-rust. "Keep interface small and stable." Stubs in enums create dead code paths and confusion. Add when implemented, not before.

### B. Rename to "disabled" status

**Rejected:** Over-engineering. If it's not implemented, it shouldn't exist. Jon Gjengset: "Clean break, no deprecated aliases."

### C. Move to separate "planned adapters" list

**Rejected:** YAGNI. The adapter system works fine with 3 adapters. When we want Codex, we'll implement it as an agent (different system entirely).
