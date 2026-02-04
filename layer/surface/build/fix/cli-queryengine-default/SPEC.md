---
type: fix
id: cli-queryengine-default
status: design
created: 2026-02-04
sessions:
  origin: 20260204-110139
related:
  - layer/surface/build/feat/mother-delivery/SPEC.md
  - layer/surface/build/feat/mother-delivery/d0-unified-search/SPEC.md
  - layer/surface/build/feat/mother-delivery/analysis-three-servers.md
beliefs:
  - mcp-is-shim-cli-is-product
  - temporal-layering-divergence
---

# fix: CLI Scry Uses QueryEngine by Default

> Make `patina scry` use the same QueryEngine pipeline that MCP uses. One pipeline, measured improvement.

## Problem

CLI and MCP return different results for the same query because they use different search pipelines:

```
$ patina scry "how should I handle errors"     # CLI: 1 result (semantic only)
$ mcp scry "how should I handle errors"        # MCP: 5 results (4 oracles + RRF)
```

**Measured baseline (session 20260204-110139):**
- CLI: 1 result from 1 oracle (semantic vector search only)
- MCP: 5 results from 4 oracles (semantic + lexical + temporal + persona, RRF fused)

The CLI uses a heuristic that picks ONE search mode (`scry_text()` or `scry_lexical()`). The MCP uses `QueryEngine` which runs all 4 oracles in parallel and fuses results with RRF.

**Who calls CLI:** Claude Code, OpenCode, and Gemini CLI all call `patina scry` via skills/bash. The CLI is the primary LLM interface, not MCP. The skills system pushes LLM usage toward CLI. MCP is legacy for local use — stdio feels clunky vs the CLI + skills approach.

**Impact:** The primary interface (CLI) gives worse results than the secondary interface (MCP). LLMs calling via skills get degraded search quality.

## Root Cause

`execute_hybrid()` in `hybrid.rs` already does the right thing — creates `QueryEngine`, runs all oracles, fuses with RRF. But it's gated behind `--hybrid` flag (added Dec 16, 2025 as experimental). The experiment is over — MCP has run QueryEngine on every query since Jan 2026 with no issues.

## Fix

Make the `(None, None, Some(query))` arm in `execute()` delegate to `execute_hybrid()` instead of the heuristic path. Special modes (`--belief`, `--file`) stay as-is.

### Before

```
execute()
  ├─ mother? → execute_via_mother()
  ├─ all_repos? → routing match
  ├─ --hybrid? → execute_hybrid()          ← opt-in QueryEngine
  └─ default match:
      ├─ --belief → scry_belief()
      ├─ --file → scry_file()
      └─ text → heuristic pick ONE oracle  ← broken path
```

### After

```
execute()
  ├─ mother? → execute_via_mother()
  ├─ all_repos? → routing match
  ├─ --belief → scry_belief()              (unchanged)
  ├─ --file → scry_file()                  (unchanged)
  └─ text → execute_hybrid()               ← always QueryEngine
```

### Files Changed

| File | Change |
|------|--------|
| `src/commands/scry/mod.rs` | Default text query delegates to `execute_hybrid()` instead of heuristic |

### What Does NOT Change

- `--hybrid` flag stays (becomes a no-op, deprecated — cleanup is D0's job)
- `--lexical` flag stays (still forces FTS5-only for debugging)
- `--dimension` flag stays (still forces specific vector dimension)
- `--belief` and `--file` modes unchanged
- `execute_hybrid()` in `hybrid.rs` unchanged
- `ScryResult` struct stays (cleanup is D0's job)
- MCP server unchanged
- Serve daemon unchanged
- No new flags, no new structs, no new files

### Escape Hatch

`--lexical` and `--dimension` still force single-oracle paths for users who need them. The old heuristic behavior is recoverable via `--dimension semantic` (equivalent to old default for semantic queries).

## Verification

**Andrew Ng approach: measure before and after with same query.**

```bash
# Before (baseline already captured):
patina scry "how should I handle errors" --limit 5
# Result: 1 hit, semantic only

# After:
patina scry "how should I handle errors" --limit 5
# Expected: 5 hits, 4 oracles, RRF fused (matching MCP output)
```

Run `patina eval` before and after to confirm no retrieval quality regression.

## Exit Criteria

- [ ] `patina scry "query"` uses QueryEngine (all oracles + RRF) by default
- [ ] `--belief` and `--file` modes unaffected
- [ ] `--lexical` and `--dimension` still work as escape hatches
- [ ] `patina eval` shows no regression
- [ ] Same query returns same result quality as MCP path

## Relationship to D0

This is the smallest measurable step of D0. It does NOT:
- Remove any flags (D0 removes `--hybrid`, `--lexical`, `--dimension`)
- Change output format (D0 migrates to FusedResult display)
- Touch MCP server (D0 makes MCP delegate to CLI)
- Touch serve daemon (D0 unifies serve path)
- Remove `ScryResult` (D0 retires it)

Those are D0 cleanup tasks. This fix just makes CLI use the same pipeline as MCP.
