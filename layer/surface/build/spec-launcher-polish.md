---
id: spec-launcher-polish
status: complete
created: 2026-01-13
completed: 2026-01-22
extracted-from: spec-init-hardening (Phase 3)
tags: [spec, launcher, mcp, ux]
references: [adapter-pattern, dependable-rust]
---

# Spec: Launcher MCP Auto-Configuration

**Problem:** If MCP configuration fails during `adapter add`, the user has to manually run `patina adapter mcp` before launching. The launcher should self-heal.

**Solution:** Launcher silently auto-configures MCP if not already configured.

**Status:** Complete ✅ (2026-01-22)

---

## Goal

Make the launcher self-healing for MCP configuration. User types `patina`, it just works.

**Principle:** Fix what you can silently, only speak when user action is needed.

---

## Implementation

### 1. Add `is_mcp_configured()` to LLMAdapter trait

In `src/adapters/mod.rs`:

```rust
/// Check if MCP is configured for this adapter
fn is_mcp_configured(&self, project_path: &Path) -> Result<bool>;
```

For Claude adapter: Check `~/.config/claude/config.json` for patina server entry.

### 2. Add silent MCP fix in launcher

In `src/commands/launch/internal.rs`, after adapter validation:

```rust
// Silent MCP auto-configuration
let adapter = adapters::get_adapter(&adapter_name)?;
if !adapter.is_mcp_configured(&project_path)? {
    // Silent fix - don't print anything on success
    let _ = adapter.configure_mcp(&project_path);
}
```

Ignore errors - if MCP config fails, user will notice when MCP tools don't work. No need to block launch.

---

## Changes Required

| File | Change |
|------|--------|
| `src/adapters/mod.rs` | Add `is_mcp_configured()` to `LLMAdapter` trait |
| `src/adapters/claude/mod.rs` | Implement: check ~/.config/claude/config.json |
| `src/adapters/gemini/mod.rs` | Implement: check appropriate config location |
| `src/adapters/opencode/mod.rs` | Implement: check appropriate config location |
| `src/commands/launch/internal.rs` | Add silent MCP auto-fix after adapter validation |

---

## Success Criteria

1. `patina` launches even if MCP wasn't configured during `adapter add`
2. No extra output on successful MCP auto-configuration
3. Launch not blocked by MCP configuration failures

---

## Non-Goals

- Changing launch output (keep the 🚀 emoji!)
- Events/observability (separate spec in deferred/)
- MCP configuration UI improvements
