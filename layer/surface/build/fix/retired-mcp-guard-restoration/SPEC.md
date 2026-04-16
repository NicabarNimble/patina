---
type: fix
id: retired-mcp-guard-restoration
status: active
created: 2026-04-15
beliefs:
  - "[[spec-driven-design]]"
  - "[[safety-boundaries]]"
related:
  - src/main.rs
  - src/commands/mother/mod.rs
  - resources/scripts/check-retired-mcp-surface.sh
  - resources/git/pre-push-checks.sh
exit_criteria:
  - id: rmgr1-main-guard-present
    text: "`src/main.rs` explicitly includes the retired MCP guard marker text (`MCP server path has been retired`) so structural invariants remain enforceable after router decomposition."
    checked: false
  - id: rmgr2-structural-script-green
    text: "`resources/scripts/check-retired-mcp-surface.sh` passes without invariant errors."
    checked: false
---

# fix: retired MCP guard restoration

## Problem

After command-router decomposition, the retired MCP rejection behavior remained in runtime dispatch, but the structural invariant guard script expects explicit retired-marker text in `src/main.rs`. The marker string drifted out of `main.rs`, causing Tier 1 structural check failure.

## Goal

Restore the explicit retired MCP guard marker in `src/main.rs` without changing CLI command/flag contracts or runtime behavior.

## Scope

- Add/restore explicit marker text in `src/main.rs` only.
- Re-run retired MCP invariant script.

## Non-goals

- No command surface changes.
- No behavior changes in serve/mother dispatch.

## Verification commands

```bash
bash resources/scripts/check-retired-mcp-surface.sh
```
